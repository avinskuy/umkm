-- Jalankan hanya pada database khusus aplikasi menggunakan principal DBA.
-- Skrip ini tidak mengatur password, membuat superuser, atau mengubah data aplikasi.

BEGIN;

DO $bootstrap$
BEGIN
    IF NOT EXISTS (SELECT 1 FROM pg_catalog.pg_roles WHERE rolname = 'usahapintar_app') THEN
        EXECUTE 'CREATE ROLE usahapintar_app NOLOGIN';
    END IF;
    IF NOT EXISTS (SELECT 1 FROM pg_catalog.pg_roles WHERE rolname = 'usahapintar_app_login') THEN
        EXECUTE 'CREATE ROLE usahapintar_app_login LOGIN';
    END IF;
    IF NOT EXISTS (SELECT 1 FROM pg_catalog.pg_roles WHERE rolname = 'usahapintar_migrator') THEN
        EXECUTE 'CREATE ROLE usahapintar_migrator LOGIN';
    END IF;
    IF NOT EXISTS (SELECT 1 FROM pg_catalog.pg_roles WHERE rolname = 'usahapintar_backup') THEN
        EXECUTE 'CREATE ROLE usahapintar_backup NOLOGIN';
    END IF;
    IF NOT EXISTS (SELECT 1 FROM pg_catalog.pg_roles WHERE rolname = 'usahapintar_backup_login') THEN
        EXECUTE 'CREATE ROLE usahapintar_backup_login LOGIN';
    END IF;
    IF NOT EXISTS (SELECT 1 FROM pg_catalog.pg_roles WHERE rolname = 'usahapintar_monitor') THEN
        EXECUTE 'CREATE ROLE usahapintar_monitor NOLOGIN';
    END IF;
    IF NOT EXISTS (SELECT 1 FROM pg_catalog.pg_roles WHERE rolname = 'usahapintar_monitor_login') THEN
        EXECUTE 'CREATE ROLE usahapintar_monitor_login LOGIN';
    END IF;
    IF NOT EXISTS (SELECT 1 FROM pg_catalog.pg_roles WHERE rolname = 'usahapintar_audit_reader') THEN
        EXECUTE 'CREATE ROLE usahapintar_audit_reader NOLOGIN';
    END IF;
END;
$bootstrap$;

-- Seluruh principal aplikasi non-superuser dan tidak boleh melewati RLS.
ALTER ROLE usahapintar_app NOLOGIN NOSUPERUSER NOCREATEDB NOCREATEROLE NOINHERIT NOREPLICATION NOBYPASSRLS;
ALTER ROLE usahapintar_app_login LOGIN INHERIT NOSUPERUSER NOCREATEDB NOCREATEROLE NOREPLICATION NOBYPASSRLS;
ALTER ROLE usahapintar_migrator LOGIN NOINHERIT NOSUPERUSER NOCREATEDB NOCREATEROLE NOREPLICATION NOBYPASSRLS;
ALTER ROLE usahapintar_backup NOLOGIN INHERIT NOSUPERUSER NOCREATEDB NOCREATEROLE NOREPLICATION NOBYPASSRLS;
ALTER ROLE usahapintar_backup_login LOGIN INHERIT NOSUPERUSER NOCREATEDB NOCREATEROLE NOREPLICATION NOBYPASSRLS;
ALTER ROLE usahapintar_monitor NOLOGIN INHERIT NOSUPERUSER NOCREATEDB NOCREATEROLE NOREPLICATION NOBYPASSRLS;
ALTER ROLE usahapintar_monitor_login LOGIN INHERIT NOSUPERUSER NOCREATEDB NOCREATEROLE NOREPLICATION NOBYPASSRLS;
ALTER ROLE usahapintar_audit_reader NOLOGIN NOINHERIT NOSUPERUSER NOCREATEDB NOCREATEROLE NOREPLICATION NOBYPASSRLS;

GRANT usahapintar_app TO usahapintar_app_login;
GRANT usahapintar_backup TO usahapintar_backup_login;
GRANT usahapintar_monitor TO usahapintar_monitor_login;

-- Pastikan principal aplikasi tidak mewarisi role bawaan atau group lain.
DO $membership_cleanup$
DECLARE
    v_member_name TEXT;
    v_role_name TEXT;
BEGIN
    FOREACH v_member_name IN ARRAY ARRAY[
        'usahapintar_app',
        'usahapintar_app_login'
    ] LOOP
        FOREACH v_role_name IN ARRAY ARRAY[
            'pg_read_all_data',
            'pg_write_all_data',
            'pg_monitor',
            'pg_signal_backend',
            'pg_read_server_files',
            'pg_write_server_files',
            'pg_execute_server_program',
            'usahapintar_backup',
            'usahapintar_monitor',
            'usahapintar_migrator'
        ] LOOP
            IF EXISTS (
                SELECT 1
                  FROM pg_catalog.pg_auth_members AS m
                  JOIN pg_catalog.pg_roles AS granted_role ON granted_role.oid = m.roleid
                  JOIN pg_catalog.pg_roles AS member_role ON member_role.oid = m.member
                 WHERE granted_role.rolname = v_role_name
                   AND member_role.rolname = v_member_name
            ) THEN
                EXECUTE pg_catalog.format(
                    'REVOKE %I FROM %I',
                    v_role_name,
                    v_member_name
                );
            END IF;
        END LOOP;
    END LOOP;
END;
$membership_cleanup$;

DO $app_membership_guard$
BEGIN
    IF EXISTS (
        SELECT 1
          FROM pg_catalog.pg_auth_members AS m
         WHERE m.member = 'usahapintar_app_login'::pg_catalog.regrole
           AND m.roleid <> 'usahapintar_app'::pg_catalog.regrole
    ) OR EXISTS (
        SELECT 1
          FROM pg_catalog.pg_auth_members AS m
         WHERE m.member = 'usahapintar_app'::pg_catalog.regrole
    ) THEN
        RAISE EXCEPTION
            'Application principals must not inherit any role except the dedicated app group';
    END IF;
END;
$app_membership_guard$;

-- Role bawaan PostgreSQL dibatasi pada tugas backup dan monitoring.
-- pg_read_all_data tidak memiliki BYPASSRLS dan tidak memberi hak tulis.
GRANT pg_read_all_data TO usahapintar_backup;
GRANT pg_monitor TO usahapintar_monitor;

DO $database_grants$
DECLARE
    v_database_name TEXT := pg_catalog.current_database();
BEGIN
    EXECUTE pg_catalog.format(
        'REVOKE CONNECT, TEMPORARY ON DATABASE %I FROM PUBLIC',
        v_database_name
    );
    EXECUTE pg_catalog.format(
        'GRANT CONNECT ON DATABASE %I TO usahapintar_app_login, usahapintar_migrator, usahapintar_backup_login, usahapintar_monitor_login',
        v_database_name
    );
END;
$database_grants$;

-- PUBLIC tidak boleh membuat objek atau membaca schema aplikasi.
REVOKE ALL ON SCHEMA public FROM PUBLIC;
GRANT USAGE ON SCHEMA public TO usahapintar_app;
GRANT USAGE, CREATE ON SCHEMA public TO usahapintar_migrator;

-- Bersihkan privilege publik yang mungkin sudah ada pada objek aplikasi.
REVOKE ALL ON ALL TABLES IN SCHEMA public FROM PUBLIC;
REVOKE ALL ON ALL SEQUENCES IN SCHEMA public FROM PUBLIC;
REVOKE EXECUTE ON ALL FUNCTIONS IN SCHEMA public FROM PUBLIC;

-- Objek baru yang dibuat oleh migrator tidak otomatis dapat diakses PUBLIC.
ALTER DEFAULT PRIVILEGES FOR ROLE usahapintar_migrator IN SCHEMA public
    REVOKE ALL ON TABLES FROM PUBLIC;
ALTER DEFAULT PRIVILEGES FOR ROLE usahapintar_migrator IN SCHEMA public
    REVOKE ALL ON SEQUENCES FROM PUBLIC;
-- EXECUTE pada function adalah default global PostgreSQL; revoke juga harus global,
-- bukan hanya per-schema.
ALTER DEFAULT PRIVILEGES FOR ROLE usahapintar_migrator
    REVOKE EXECUTE ON FUNCTIONS FROM PUBLIC;

COMMIT;
