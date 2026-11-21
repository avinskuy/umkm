-- Identity Core slice 0002: contact verification sessions, OTP challenges and events.
-- Cryptographic key material and runtime/provider integration are intentionally external.

CREATE TABLE public.verification_sessions (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id UUID,
    purpose TEXT NOT NULL,
    channel TEXT NOT NULL,
    destination_ciphertext BYTEA NOT NULL,
    destination_nonce BYTEA NOT NULL,
    encryption_key_version TEXT NOT NULL,
    destination_hmac BYTEA NOT NULL,
    hmac_key_version TEXT NOT NULL,
    status TEXT NOT NULL DEFAULT 'pending',
    failed_attempt_count SMALLINT NOT NULL DEFAULT 0,
    locked_until TIMESTAMPTZ,
    resend_available_at TIMESTAMPTZ NOT NULL DEFAULT clock_timestamp(),
    created_at TIMESTAMPTZ NOT NULL DEFAULT clock_timestamp(),
    expires_at TIMESTAMPTZ NOT NULL,
    verified_at TIMESTAMPTZ,
    expired_at TIMESTAMPTZ,
    cancelled_at TIMESTAMPTZ,

    CONSTRAINT verification_sessions_user_fk
        FOREIGN KEY (user_id) REFERENCES public.users (id) ON DELETE RESTRICT,
    CONSTRAINT verification_sessions_purpose_check
        CHECK (purpose IN ('registration', 'primary_contact_change', 'recovery_contact_add', 'account_recovery')),
    CONSTRAINT verification_sessions_channel_check
        CHECK (channel IN ('email', 'whatsapp')),
    CONSTRAINT verification_sessions_user_purpose_check
        CHECK (
            (purpose = 'account_recovery')
            OR (purpose IN ('registration', 'primary_contact_change', 'recovery_contact_add') AND user_id IS NOT NULL)
        ),
    CONSTRAINT verification_sessions_payload_check
        CHECK (
            octet_length(destination_ciphertext) > 0
            AND octet_length(destination_nonce) > 0
            AND length(btrim(encryption_key_version)) > 0
            AND octet_length(destination_hmac) > 0
            AND length(btrim(hmac_key_version)) > 0
        ),
    CONSTRAINT verification_sessions_status_check
        CHECK (status IN ('pending', 'verified', 'expired', 'cancelled')),
    CONSTRAINT verification_sessions_attempt_count_check
        CHECK (failed_attempt_count BETWEEN 0 AND 5),
    CONSTRAINT verification_sessions_lock_check
        CHECK (
            (failed_attempt_count = 5 AND locked_until IS NOT NULL)
            OR (failed_attempt_count < 5 AND locked_until IS NULL)
        ),
    CONSTRAINT verification_sessions_deadline_check
        CHECK (expires_at > created_at),
    CONSTRAINT verification_sessions_terminal_timestamps_check
        CHECK (
            (status = 'pending' AND verified_at IS NULL AND expired_at IS NULL AND cancelled_at IS NULL)
            OR (status = 'verified' AND verified_at IS NOT NULL AND expired_at IS NULL AND cancelled_at IS NULL)
            OR (status = 'expired' AND verified_at IS NULL AND expired_at IS NOT NULL AND cancelled_at IS NULL)
            OR (status = 'cancelled' AND verified_at IS NULL AND expired_at IS NULL AND cancelled_at IS NOT NULL)
        )
);

CREATE INDEX verification_sessions_user_purpose_status_idx
    ON public.verification_sessions (user_id, purpose, status, expires_at);

CREATE INDEX verification_sessions_expiry_idx
    ON public.verification_sessions (expires_at)
    WHERE status = 'pending';

CREATE FUNCTION public.guard_verification_session()
RETURNS TRIGGER
LANGUAGE plpgsql
SET search_path = pg_catalog
AS $$
DECLARE
    v_account_status TEXT;
    v_initial_channel TEXT;
    v_pending_expires_at TIMESTAMPTZ;
BEGIN
    IF TG_OP = 'UPDATE' THEN
        IF NEW.id IS DISTINCT FROM OLD.id
           OR NEW.purpose IS DISTINCT FROM OLD.purpose
           OR NEW.channel IS DISTINCT FROM OLD.channel
           OR NEW.destination_ciphertext IS DISTINCT FROM OLD.destination_ciphertext
           OR NEW.destination_nonce IS DISTINCT FROM OLD.destination_nonce
           OR NEW.encryption_key_version IS DISTINCT FROM OLD.encryption_key_version
           OR NEW.destination_hmac IS DISTINCT FROM OLD.destination_hmac
           OR NEW.hmac_key_version IS DISTINCT FROM OLD.hmac_key_version
           OR NEW.created_at IS DISTINCT FROM OLD.created_at
           OR NEW.expires_at IS DISTINCT FROM OLD.expires_at THEN
            RAISE EXCEPTION 'verification session identity and destination are immutable'
                USING ERRCODE = '23514';
        END IF;

        IF OLD.status <> 'pending'
           AND NOT (
               OLD.status = 'verified'
               AND OLD.purpose = 'account_recovery'
               AND OLD.user_id IS NULL
               AND NEW.user_id IS NOT NULL
               AND NEW.id IS NOT DISTINCT FROM OLD.id
               AND NEW.purpose IS NOT DISTINCT FROM OLD.purpose
               AND NEW.channel IS NOT DISTINCT FROM OLD.channel
               AND NEW.destination_ciphertext IS NOT DISTINCT FROM OLD.destination_ciphertext
               AND NEW.destination_nonce IS NOT DISTINCT FROM OLD.destination_nonce
               AND NEW.encryption_key_version IS NOT DISTINCT FROM OLD.encryption_key_version
               AND NEW.destination_hmac IS NOT DISTINCT FROM OLD.destination_hmac
               AND NEW.hmac_key_version IS NOT DISTINCT FROM OLD.hmac_key_version
               AND NEW.status IS NOT DISTINCT FROM OLD.status
               AND NEW.failed_attempt_count IS NOT DISTINCT FROM OLD.failed_attempt_count
               AND NEW.locked_until IS NOT DISTINCT FROM OLD.locked_until
               AND NEW.resend_available_at IS NOT DISTINCT FROM OLD.resend_available_at
               AND NEW.created_at IS NOT DISTINCT FROM OLD.created_at
               AND NEW.expires_at IS NOT DISTINCT FROM OLD.expires_at
               AND NEW.verified_at IS NOT DISTINCT FROM OLD.verified_at
               AND NEW.expired_at IS NOT DISTINCT FROM OLD.expired_at
               AND NEW.cancelled_at IS NOT DISTINCT FROM OLD.cancelled_at
           ) THEN
            RAISE EXCEPTION 'terminal verification session cannot be changed'
                USING ERRCODE = '23514';
        END IF;

        IF NEW.status IS DISTINCT FROM OLD.status
           AND NEW.status NOT IN ('verified', 'expired', 'cancelled') THEN
            RAISE EXCEPTION 'invalid verification session transition'
                USING ERRCODE = '23514';
        END IF;

        IF NEW.status IS DISTINCT FROM OLD.status AND NEW.status = 'verified' AND OLD.expires_at <= clock_timestamp() THEN
            RAISE EXCEPTION 'expired verification session cannot be verified'
                USING ERRCODE = '23514';
        END IF;

        IF NEW.status IS DISTINCT FROM OLD.status AND NEW.status = 'expired' AND OLD.expires_at > clock_timestamp() THEN
            RAISE EXCEPTION 'verification session cannot expire before its deadline'
                USING ERRCODE = '23514';
        END IF;

        IF OLD.failed_attempt_count = 5
           AND NEW.failed_attempt_count = 5
           AND NEW.locked_until < OLD.locked_until THEN
            RAISE EXCEPTION 'verification lock deadline cannot be shortened'
                USING ERRCODE = '23514';
        END IF;

        IF OLD.user_id IS NOT NULL AND NEW.user_id IS DISTINCT FROM OLD.user_id THEN
            RAISE EXCEPTION 'verification session account binding is immutable'
                USING ERRCODE = '23514';
        END IF;

        IF OLD.user_id IS NULL AND NEW.user_id IS NOT NULL
           AND (OLD.purpose <> 'account_recovery' OR OLD.status <> 'verified' OR NEW.status <> 'verified') THEN
            RAISE EXCEPTION 'recovery account may only be assigned once after contact verification'
                USING ERRCODE = '23514';
        END IF;

        IF NEW.failed_attempt_count < OLD.failed_attempt_count
           AND NOT (
               OLD.status = 'pending'
               AND OLD.locked_until IS NOT NULL
               AND OLD.locked_until <= clock_timestamp()
               AND NEW.failed_attempt_count = 0
               AND NEW.locked_until IS NULL
           ) THEN
            RAISE EXCEPTION 'verification attempt counter can only reset after its lock expires'
                USING ERRCODE = '23514';
        END IF;
    END IF;

    IF NEW.purpose = 'registration' THEN
        SELECT account_status, initial_contact_channel, pending_expires_at
        INTO v_account_status, v_initial_channel, v_pending_expires_at
        FROM public.users
        WHERE id = NEW.user_id;

        IF NOT FOUND OR v_account_status <> 'pending_verification' THEN
            RAISE EXCEPTION 'registration verification requires a pending account'
                USING ERRCODE = '23514';
        END IF;

        IF (NEW.channel = 'email' AND v_initial_channel <> 'email')
           OR (NEW.channel = 'whatsapp' AND v_initial_channel <> 'phone') THEN
            RAISE EXCEPTION 'verification channel must match the initial registration contact'
                USING ERRCODE = '23514';
        END IF;

        IF NEW.expires_at > v_pending_expires_at THEN
            RAISE EXCEPTION 'registration verification cannot outlive pending account'
                USING ERRCODE = '23514';
        END IF;
    END IF;

    RETURN NEW;
END;
$$;

CREATE TRIGGER verification_sessions_10_guard
    BEFORE INSERT OR UPDATE ON public.verification_sessions
    FOR EACH ROW EXECUTE FUNCTION public.guard_verification_session();

CREATE TABLE public.verification_challenges (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    verification_session_id UUID NOT NULL,
    code_hash BYTEA NOT NULL,
    hash_key_version TEXT NOT NULL,
    status TEXT NOT NULL DEFAULT 'pending',
    created_at TIMESTAMPTZ NOT NULL DEFAULT clock_timestamp(),
    expires_at TIMESTAMPTZ NOT NULL,
    consumed_at TIMESTAMPTZ,
    expired_at TIMESTAMPTZ,
    superseded_at TIMESTAMPTZ,
    cancelled_at TIMESTAMPTZ,

    CONSTRAINT verification_challenges_session_fk
        FOREIGN KEY (verification_session_id)
        REFERENCES public.verification_sessions (id) ON DELETE RESTRICT,
    CONSTRAINT verification_challenges_code_hash_check
        CHECK (octet_length(code_hash) > 0 AND length(btrim(hash_key_version)) > 0),
    CONSTRAINT verification_challenges_status_check
        CHECK (status IN ('pending', 'consumed', 'expired', 'superseded', 'cancelled')),
    CONSTRAINT verification_challenges_deadline_check
        CHECK (expires_at > created_at AND expires_at <= created_at + INTERVAL '10 minutes'),
    CONSTRAINT verification_challenges_terminal_timestamps_check
        CHECK (
            (status = 'pending' AND consumed_at IS NULL AND expired_at IS NULL AND superseded_at IS NULL AND cancelled_at IS NULL)
            OR (status = 'consumed' AND consumed_at IS NOT NULL AND expired_at IS NULL AND superseded_at IS NULL AND cancelled_at IS NULL)
            OR (status = 'expired' AND consumed_at IS NULL AND expired_at IS NOT NULL AND superseded_at IS NULL AND cancelled_at IS NULL)
            OR (status = 'superseded' AND consumed_at IS NULL AND expired_at IS NULL AND superseded_at IS NOT NULL AND cancelled_at IS NULL)
            OR (status = 'cancelled' AND consumed_at IS NULL AND expired_at IS NULL AND superseded_at IS NULL AND cancelled_at IS NOT NULL)
        ),
    CONSTRAINT verification_challenges_id_session_uidx UNIQUE (id, verification_session_id)
);

CREATE UNIQUE INDEX verification_challenges_one_pending_per_session_uidx
    ON public.verification_challenges (verification_session_id)
    WHERE status = 'pending';

CREATE INDEX verification_challenges_expiry_idx
    ON public.verification_challenges (expires_at)
    WHERE status = 'pending';

CREATE FUNCTION public.guard_verification_challenge()
RETURNS TRIGGER
LANGUAGE plpgsql
SET search_path = pg_catalog
AS $$
DECLARE
    v_session_status TEXT;
    v_session_expires_at TIMESTAMPTZ;
BEGIN
    IF TG_OP = 'INSERT' THEN
        SELECT status, expires_at
        INTO v_session_status, v_session_expires_at
        FROM public.verification_sessions
        WHERE id = NEW.verification_session_id;

        IF NOT FOUND OR v_session_status <> 'pending' OR NEW.expires_at > v_session_expires_at THEN
            RAISE EXCEPTION 'challenge requires a pending session and cannot outlive it'
                USING ERRCODE = '23514';
        END IF;
    END IF;

    IF TG_OP = 'UPDATE' THEN
        IF NEW.id IS DISTINCT FROM OLD.id
           OR NEW.verification_session_id IS DISTINCT FROM OLD.verification_session_id
           OR NEW.code_hash IS DISTINCT FROM OLD.code_hash
           OR NEW.hash_key_version IS DISTINCT FROM OLD.hash_key_version
           OR NEW.created_at IS DISTINCT FROM OLD.created_at
           OR NEW.expires_at IS DISTINCT FROM OLD.expires_at THEN
            RAISE EXCEPTION 'verification challenge identity and code hash are immutable'
                USING ERRCODE = '23514';
        END IF;

        IF OLD.status <> 'pending' THEN
            RAISE EXCEPTION 'terminal verification challenge cannot be changed'
                USING ERRCODE = '23514';
        END IF;

        IF NEW.status NOT IN ('consumed', 'expired', 'superseded', 'cancelled') THEN
            RAISE EXCEPTION 'invalid verification challenge transition'
                USING ERRCODE = '23514';
        END IF;

        IF NEW.status = 'consumed' AND OLD.expires_at <= clock_timestamp() THEN
            RAISE EXCEPTION 'expired verification challenge cannot be consumed'
                USING ERRCODE = '23514';
        END IF;

        IF NEW.status = 'expired' AND OLD.expires_at > clock_timestamp() THEN
            RAISE EXCEPTION 'verification challenge cannot expire before its deadline'
                USING ERRCODE = '23514';
        END IF;
    END IF;

    RETURN NEW;
END;
$$;

CREATE TRIGGER verification_challenges_10_guard
    BEFORE INSERT OR UPDATE ON public.verification_challenges
    FOR EACH ROW EXECUTE FUNCTION public.guard_verification_challenge();

CREATE TABLE public.otp_verification_attempt_events (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    verification_session_id UUID NOT NULL,
    verification_challenge_id UUID NOT NULL,
    destination_hmac BYTEA NOT NULL,
    hmac_key_version TEXT NOT NULL,
    channel TEXT NOT NULL,
    purpose TEXT NOT NULL,
    outcome TEXT NOT NULL,
    attempted_at TIMESTAMPTZ NOT NULL DEFAULT clock_timestamp(),

    CONSTRAINT otp_verification_attempt_events_session_fk
        FOREIGN KEY (verification_session_id)
        REFERENCES public.verification_sessions (id) ON DELETE RESTRICT,
    CONSTRAINT otp_verification_attempt_events_challenge_session_fk
        FOREIGN KEY (verification_challenge_id, verification_session_id)
        REFERENCES public.verification_challenges (id, verification_session_id) ON DELETE RESTRICT,
    CONSTRAINT otp_verification_attempt_events_fingerprint_check
        CHECK (octet_length(destination_hmac) > 0 AND length(btrim(hmac_key_version)) > 0),
    CONSTRAINT otp_verification_attempt_events_channel_check
        CHECK (channel IN ('email', 'whatsapp')),
    CONSTRAINT otp_verification_attempt_events_purpose_check
        CHECK (purpose IN ('registration', 'primary_contact_change', 'recovery_contact_add', 'account_recovery')),
    CONSTRAINT otp_verification_attempt_events_outcome_check
        CHECK (outcome IN ('failed', 'succeeded'))
);

CREATE INDEX otp_verification_attempt_events_contact_time_idx
    ON public.otp_verification_attempt_events (destination_hmac, channel, attempted_at);

CREATE INDEX otp_verification_attempt_events_session_idx
    ON public.otp_verification_attempt_events (verification_session_id, attempted_at);

CREATE FUNCTION public.guard_otp_verification_attempt_event()
RETURNS TRIGGER
LANGUAGE plpgsql
SET search_path = pg_catalog
AS $$
DECLARE
    v_destination_hmac BYTEA;
    v_hmac_key_version TEXT;
    v_channel TEXT;
    v_purpose TEXT;
    v_session_status TEXT;
    v_session_expires_at TIMESTAMPTZ;
    v_session_locked_until TIMESTAMPTZ;
    v_challenge_status TEXT;
    v_challenge_expires_at TIMESTAMPTZ;
BEGIN
    IF TG_OP <> 'INSERT' THEN
        RAISE EXCEPTION 'OTP verification attempt events cannot be updated'
            USING ERRCODE = '23514';
    END IF;

    SELECT destination_hmac, hmac_key_version, channel, purpose, status, expires_at, locked_until
    INTO v_destination_hmac, v_hmac_key_version, v_channel, v_purpose,
         v_session_status, v_session_expires_at, v_session_locked_until
    FROM public.verification_sessions
    WHERE id = NEW.verification_session_id;

    IF NOT FOUND
       OR NEW.destination_hmac IS DISTINCT FROM v_destination_hmac
       OR NEW.hmac_key_version IS DISTINCT FROM v_hmac_key_version
       OR NEW.channel IS DISTINCT FROM v_channel
       OR NEW.purpose IS DISTINCT FROM v_purpose THEN
        RAISE EXCEPTION 'OTP attempt event must match its verification session'
            USING ERRCODE = '23514';
    END IF;

    IF v_session_status <> 'pending'
       OR v_session_expires_at <= clock_timestamp()
       OR (v_session_locked_until IS NOT NULL AND v_session_locked_until > clock_timestamp()) THEN
        RAISE EXCEPTION 'OTP attempt event requires an eligible verification session'
            USING ERRCODE = '23514';
    END IF;

    SELECT status, expires_at
    INTO v_challenge_status, v_challenge_expires_at
    FROM public.verification_challenges
    WHERE id = NEW.verification_challenge_id
      AND verification_session_id = NEW.verification_session_id;

    IF v_challenge_status IS DISTINCT FROM 'pending'
       OR v_challenge_expires_at <= clock_timestamp() THEN
        RAISE EXCEPTION 'OTP attempt event requires an unexpired pending challenge'
            USING ERRCODE = '23514';
    END IF;

    RETURN NEW;
END;
$$;

CREATE TRIGGER otp_verification_attempt_events_10_guard
    BEFORE INSERT OR UPDATE ON public.otp_verification_attempt_events
    FOR EACH ROW EXECUTE FUNCTION public.guard_otp_verification_attempt_event();

CREATE TABLE public.otp_delivery_events (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    destination_hmac BYTEA NOT NULL,
    hmac_key_version TEXT NOT NULL,
    channel TEXT NOT NULL,
    purpose TEXT NOT NULL,
    dispatch_status TEXT NOT NULL DEFAULT 'pending',
    created_at TIMESTAMPTZ NOT NULL DEFAULT clock_timestamp(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT clock_timestamp(),

    CONSTRAINT otp_delivery_events_fingerprint_check
        CHECK (octet_length(destination_hmac) > 0 AND length(btrim(hmac_key_version)) > 0),
    CONSTRAINT otp_delivery_events_channel_check
        CHECK (channel IN ('email', 'whatsapp')),
    CONSTRAINT otp_delivery_events_purpose_check
        CHECK (purpose IN ('registration', 'primary_contact_change', 'recovery_contact_add', 'account_recovery')),
    CONSTRAINT otp_delivery_events_status_check
        CHECK (dispatch_status IN ('pending', 'accepted', 'failed', 'unknown')),
    CONSTRAINT otp_delivery_events_timestamp_check
        CHECK (updated_at >= created_at)
);

CREATE INDEX otp_delivery_events_contact_time_idx
    ON public.otp_delivery_events (destination_hmac, channel, created_at);

CREATE FUNCTION public.guard_otp_delivery_event()
RETURNS TRIGGER
LANGUAGE plpgsql
SET search_path = pg_catalog
AS $$
BEGIN
    IF NEW.id IS DISTINCT FROM OLD.id
       OR NEW.destination_hmac IS DISTINCT FROM OLD.destination_hmac
       OR NEW.hmac_key_version IS DISTINCT FROM OLD.hmac_key_version
       OR NEW.channel IS DISTINCT FROM OLD.channel
       OR NEW.purpose IS DISTINCT FROM OLD.purpose
       OR NEW.created_at IS DISTINCT FROM OLD.created_at THEN
        RAISE EXCEPTION 'OTP delivery event identity is immutable'
            USING ERRCODE = '23514';
    END IF;

    IF OLD.dispatch_status <> 'pending'
       OR NEW.dispatch_status NOT IN ('accepted', 'failed', 'unknown') THEN
        RAISE EXCEPTION 'OTP delivery event has an invalid status transition'
            USING ERRCODE = '23514';
    END IF;

    NEW.updated_at := clock_timestamp();
    RETURN NEW;
END;
$$;

CREATE TRIGGER otp_delivery_events_10_guard
    BEFORE UPDATE ON public.otp_delivery_events
    FOR EACH ROW EXECUTE FUNCTION public.guard_otp_delivery_event();

REVOKE ALL ON TABLE
    public.verification_sessions,
    public.verification_challenges,
    public.otp_verification_attempt_events,
    public.otp_delivery_events
FROM PUBLIC;

REVOKE ALL ON FUNCTION public.guard_verification_session() FROM PUBLIC;
REVOKE ALL ON FUNCTION public.guard_verification_challenge() FROM PUBLIC;
REVOKE ALL ON FUNCTION public.guard_otp_verification_attempt_event() FROM PUBLIC;
REVOKE ALL ON FUNCTION public.guard_otp_delivery_event() FROM PUBLIC;

COMMENT ON TABLE public.verification_sessions IS
    'Sesi verifikasi kontak; menyimpan tujuan terenkripsi dan fingerprint HMAC tanpa key material.';
COMMENT ON TABLE public.verification_challenges IS
    'Challenge OTP keyed-hash dengan masa berlaku maksimal 10 menit dan penggunaan satu kali.';
COMMENT ON TABLE public.otp_verification_attempt_events IS
    'Jejak append-only percobaan kode yang diproses; batas lintas sesi ditentukan oleh kebijakan runtime.';
COMMENT ON TABLE public.otp_delivery_events IS
    'Event pengiriman OTP lintas sesi untuk rate limit; tidak menyimpan tujuan plaintext atau kode.';
