import { useEffect, useState, type FormEvent } from "react";
import styles from "./ResetPasswordPage.module.css";

type ApiMessage = {
  message?: string;
  error?: string;
};

export function ResetPasswordPage() {
  const [token, setToken] = useState<string | null>(null);
  const [tokenReady, setTokenReady] = useState(false);
  const [password, setPassword] = useState("");
  const [confirmation, setConfirmation] = useState("");
  const [pending, setPending] = useState(false);
  const [notice, setNotice] = useState("");
  const [completed, setCompleted] = useState(false);
  const submitDisabled = pending || !password || !confirmation;

  useEffect(() => {
    const fragment = new URLSearchParams(window.location.hash.slice(1));
    const resetToken = fragment.get("token");
    if (resetToken && /^[a-f0-9]{64}$/i.test(resetToken)) {
      setToken(resetToken);
      window.history.replaceState(
        null,
        "",
        window.location.pathname + window.location.search,
      );
    }
    setTokenReady(true);
  }, []);

  async function submit(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    setNotice("");

    if (!token) {
      setNotice("Tautan reset tidak valid atau sudah kedaluwarsa.");
      return;
    }
    if (password !== confirmation) {
      setNotice("Konfirmasi password belum cocok.");
      return;
    }

    setPending(true);
    try {
      const response = await fetch("/api/v1/identity/recovery/password-reset", {
        method: "POST",
        credentials: "same-origin",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({
          token,
          new_password: password,
          confirm_password: confirmation,
        }),
      });
      let payload: ApiMessage | null = null;
      try {
        payload = (await response.json()) as ApiMessage;
      } catch {
        payload = null;
      }

      if (!response.ok) {
        setNotice(
          payload?.error ?? "Reset password gagal. Periksa tautan atau coba lagi.",
        );
        return;
      }

      setToken(null);
      setPassword("");
      setConfirmation("");
      setCompleted(true);
      setNotice(payload?.message ?? "Password diperbarui. Silakan masuk kembali.");
    } catch {
      setNotice("Koneksi belum tersedia. Periksa jaringan lalu coba lagi.");
    } finally {
      setPending(false);
    }
  }

  if (!tokenReady) {
    return (
      <main className={styles.page} aria-busy="true">
        <p className={styles.eyebrow}>USAHAPINTAR · KEAMANAN AKUN</p>
        <h1>Memeriksa tautan reset</h1>
      </main>
    );
  }

  return (
    <main className={styles.page}>
      <section className={styles.panel} aria-labelledby="reset-title">
        <a className={styles.brand} href="/" aria-label="USahapintar beranda">
          <span className={styles.brandMark} aria-hidden="true">
            U
          </span>
          <span>USahapintar</span>
        </a>

        <p className={styles.eyebrow}>KEAMANAN AKUN</p>
        <h1 id="reset-title">Buat password baru</h1>
        <p className={styles.description}>
          Gunakan password baru untuk mengamankan akun. Setelah berhasil, semua sesi
          sebelumnya akan berakhir dan kamu perlu masuk kembali.
        </p>

        {completed ? (
          <div className={styles.success} role="status">
            {notice}
          </div>
        ) : !token ? (
          <div className={styles.notice} role="alert">
            Tautan reset tidak valid atau sudah kedaluwarsa. Minta tautan pemulihan baru
            melalui alur resmi USahapintar.
          </div>
        ) : (
          <form className={styles.form} onSubmit={submit}>
            <label htmlFor="new-password">Password baru</label>
            <input
              id="new-password"
              type="password"
              autoComplete="new-password"
              required
              maxLength={1024}
              value={password}
              onChange={(event) => setPassword(event.target.value)}
              disabled={pending}
            />

            <label htmlFor="confirm-password">Konfirmasi password baru</label>
            <input
              id="confirm-password"
              type="password"
              autoComplete="new-password"
              required
              maxLength={1024}
              value={confirmation}
              onChange={(event) => setConfirmation(event.target.value)}
              disabled={pending}
            />

            <p className={styles.hint}>
              Password harus memenuhi kebijakan keamanan akun. Jangan gunakan password
              yang pernah dipakai pada layanan lain.
            </p>

            {notice && (
              <p className={styles.notice} role="alert">
                {notice}
              </p>
            )}

            <button type="submit" disabled={submitDisabled}>
              {pending ? "Menyimpan..." : "Simpan password baru"}
            </button>
          </form>
        )}

        <a className={styles.backLink} href="/">
          Kembali ke beranda
        </a>
      </section>
    </main>
  );
}
