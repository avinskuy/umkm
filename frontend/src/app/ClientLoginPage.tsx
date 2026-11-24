import { useState, type FormEvent } from "react";
import {
  getCurrentClient,
  IdentityApiError,
  loginClient,
  type ClientRole,
} from "./identityApi";
import styles from "./ClientLoginPage.module.css";

const roleOptions: { value: ClientRole; label: string; description: string }[] = [
  {
    value: "business_owner",
    label: "Pemilik Usaha",
    description: "Kembangkan usaha dan temukan peluang pendanaan.",
  },
  {
    value: "investor",
    label: "Investor",
    description: "Temukan usaha potensial dan bangun koneksi.",
  },
];

export function ClientLoginPage() {
  const [role, setRole] = useState<ClientRole>("business_owner");
  const [identifier, setIdentifier] = useState("");
  const [password, setPassword] = useState("");
  const [pending, setPending] = useState(false);
  const [notice, setNotice] = useState("");
  const [noticeKind, setNoticeKind] = useState<"error" | "success">("error");

  async function submit(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    setNotice("");
    setPending(true);

    try {
      await loginClient(role, identifier.trim(), password);
      await getCurrentClient(role);
      setPassword("");
      setNoticeKind("success");
      setNotice(
        role === "business_owner"
          ? "Berhasil masuk sebagai Pemilik Usaha."
          : "Berhasil masuk sebagai Investor.",
      );
    } catch (error) {
      setNoticeKind("error");
      setNotice(
        error instanceof IdentityApiError
          ? error.message
          : "Login belum berhasil. Silakan coba lagi.",
      );
    } finally {
      setPending(false);
    }
  }

  return (
    <main className={styles.page}>
      <section className={styles.panel} aria-labelledby="client-login-title">
        <a className={styles.brand} href="/" aria-label="USahapintar beranda">
          <span className={styles.brandMark} aria-hidden="true">
            U
          </span>
          <span>USahapintar</span>
        </a>

        <p className={styles.eyebrow}>RUANG KONEKSI USAHA</p>
        <h1 id="client-login-title">Selamat datang kembali</h1>
        <p className={styles.description}>
          Masuk untuk melanjutkan perjalanan dan membuka peluang baru.
        </p>

        <div className={styles.rolePicker} role="group" aria-label="Jenis akun">
          {roleOptions.map((option) => (
            <button
              className={role === option.value ? styles.roleActive : styles.roleOption}
              key={option.value}
              type="button"
              aria-pressed={role === option.value}
              onClick={() => {
                setRole(option.value);
                setNotice("");
              }}
              disabled={pending}
            >
              <span className={styles.roleLabel}>{option.label}</span>
              <span className={styles.roleDescription}>{option.description}</span>
            </button>
          ))}
        </div>

        <form className={styles.form} onSubmit={submit} aria-busy={pending}>
          <label htmlFor="client-identifier">Email atau nomor WhatsApp</label>
          <input
            id="client-identifier"
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

          <label htmlFor="client-password">Password</label>
          <input
            id="client-password"
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
              className={noticeKind === "success" ? styles.success : styles.notice}
              role={noticeKind === "success" ? "status" : "alert"}
            >
              {notice}
            </p>
          )}

          <button
            className={styles.submit}
            type="submit"
            disabled={pending || !identifier.trim() || !password}
          >
            {pending ? "Memeriksa..." : "Masuk"}
          </button>
        </form>

        <p className={styles.registerPrompt}>
          Belum punya akun? <a href="/register">Daftar sekarang</a>
        </p>
        <a className={styles.backLink} href="/">
          Kembali ke beranda
        </a>
      </section>
    </main>
  );
}
