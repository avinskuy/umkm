import { useState, type FormEvent } from "react";
import {
  beginAdminLogin,
  completeAdminLogin,
  IdentityApiError,
} from "./identityApi";
import { navigateTo } from "./navigation";
import styles from "./AdminLoginPage.module.css";

type LoginPhase = "password" | "totp";

export function AdminLoginPage() {
  const [phase, setPhase] = useState<LoginPhase>("password");
  const [identifier, setIdentifier] = useState("");
  const [password, setPassword] = useState("");
  const [code, setCode] = useState("");
  const [pending, setPending] = useState(false);
  const [notice, setNotice] = useState("");
  const [noticeKind, setNoticeKind] = useState<"error" | "success">("error");

  async function submitPassword(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    setNotice("");
    setPending(true);

    try {
      await beginAdminLogin(identifier.trim(), password);
      setPassword("");
      setCode("");
      setPhase("totp");
      setNoticeKind("success");
      setNotice(
        "Password diterima. Masukkan kode authenticator untuk melanjutkan.",
      );
    } catch (error) {
      setNoticeKind("error");
      setNotice(
        error instanceof IdentityApiError
          ? error.message
          : "Login Admin belum berhasil. Coba lagi.",
      );
    } finally {
      setPending(false);
    }
  }

  async function submitTotp(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    setNotice("");
    setPending(true);

    try {
      await completeAdminLogin(code);
      navigateTo("/admin");
    } catch (error) {
      setNoticeKind("error");
      setNotice(
        error instanceof IdentityApiError
          ? error.message
          : "Verifikasi authenticator belum berhasil. Coba lagi.",
      );
    } finally {
      setPending(false);
    }
  }

  function returnToPassword() {
    setCode("");
    setNotice("");
    setNoticeKind("error");
    setPhase("password");
  }

  return (
    <main className={styles.page}>
      <section className={styles.panel} aria-labelledby="admin-login-title">
        <a className={styles.brand} href="/" aria-label="USahapintar beranda">
          <span className={styles.brandMark} aria-hidden="true">
            U
          </span>
          <span>USahapintar</span>
        </a>

        <p className={styles.eyebrow}>AREA ADMINISTRATOR</p>
        <h1 id="admin-login-title">Admin sign in</h1>
        <p className={styles.description}>
          Akses khusus administrator dengan verifikasi dua langkah.
        </p>

        <div className={styles.stepIndicator} aria-label="Tahap autentikasi">
          <span
            className={
              phase === "password" ? styles.activeStep : styles.completeStep
            }
          >
            1 <span>Password</span>
          </span>
          <span className={styles.stepLine} aria-hidden="true" />
          <span
            className={
              phase === "totp" ? styles.activeStep : styles.pendingStep
            }
          >
            2 <span>Authenticator</span>
          </span>
        </div>

        {phase === "password" ? (
          <form
            className={styles.form}
            onSubmit={submitPassword}
            aria-busy={pending}
          >
            <label htmlFor="admin-identifier">Email atau nomor terdaftar</label>
            <input
              id="admin-identifier"
              name="identifier"
              type="text"
              autoComplete="username"
              autoCapitalize="none"
              spellCheck={false}
              maxLength={254}
              required
              value={identifier}
              onChange={(event) => setIdentifier(event.target.value)}
              disabled={pending}
            />

            <label htmlFor="admin-password">Password</label>
            <input
              id="admin-password"
              name="password"
              type="password"
              autoComplete="current-password"
              maxLength={1024}
              required
              value={password}
              onChange={(event) => setPassword(event.target.value)}
              disabled={pending}
            />

            {notice && (
              <p
                className={
                  noticeKind === "success" ? styles.success : styles.notice
                }
                role={noticeKind === "success" ? "status" : "alert"}
              >
                {notice}
              </p>
            )}

            <button
              type="submit"
              disabled={pending || !identifier.trim() || !password}
            >
              {pending ? "Memeriksa..." : "Lanjutkan"}
            </button>
          </form>
        ) : (
          <form
            className={styles.form}
            onSubmit={submitTotp}
            aria-busy={pending}
          >
            <p className={styles.stepDescription}>
              Buka aplikasi authenticator yang terdaftar dan masukkan kode enam digit.
              Challenge hanya berlaku dalam waktu singkat.
            </p>

            <label htmlFor="admin-totp-code">Kode authenticator</label>
            <input
              id="admin-totp-code"
              name="code"
              type="text"
              inputMode="numeric"
              autoComplete="one-time-code"
              pattern="[0-9]{6}"
              maxLength={6}
              minLength={6}
              required
              autoFocus
              value={code}
              onChange={(event) =>
                setCode(event.target.value.replace(/\D/g, "").slice(0, 6))
              }
              disabled={pending}
            />

            {notice && (
              <p className={styles.notice} role="alert">
                {notice}
              </p>
            )}

            <button type="submit" disabled={pending || code.length !== 6}>
              {pending ? "Memverifikasi..." : "Verifikasi dan masuk"}
            </button>
            <button
              className={styles.secondaryButton}
              type="button"
              onClick={returnToPassword}
              disabled={pending}
            >
              Kembali ke login password
            </button>
          </form>
        )}

        <p className={styles.securityNote}>
          Sesi Admin menggunakan cookie aman terpisah dan akan berakhir otomatis.
        </p>
        <a className={styles.backLink} href="/">
          Kembali ke beranda
        </a>
      </section>
    </main>
  );
}
