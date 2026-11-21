-- Jalankan hanya pada database khusus aplikasi menggunakan principal DBA.
-- Skrip ini tidak mengatur password, membuat superuser, atau mengubah data aplikasi.

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
ALTER DEFAULT PRIVILEGES FOR ROLE usahapintar_migrator IN SCHEMA public
    REVOKE EXECUTE ON FUNCTIONS FROM PUBLIC;
