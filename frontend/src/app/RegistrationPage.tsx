import { useState } from "react";
import type { ClientRole } from "./identityApi";
import styles from "./RegistrationPage.module.css";

type ContactChannel = "email" | "whatsapp";

const roleOptions: {
  value: ClientRole;
  label: string;
  description: string;
}[] = [
  {
    value: "business_owner",
    label: "Pemilik Usaha",
    description: "Perkenalkan usaha dan temukan peluang pendanaan.",
  },
  {
    value: "investor",
    label: "Investor",
    description: "Temukan usaha potensial untuk dikembangkan bersama.",
  },
];

export function RegistrationPage() {
  const [role, setRole] = useState<ClientRole>("business_owner");
  const [channel, setChannel] = useState<ContactChannel>("email");

  return (
    <main className={styles.page}>
      <section className={styles.panel} aria-labelledby="registration-title">
        <a className={styles.brand} href="/" aria-label="USahapintar beranda">
          <span className={styles.brandMark} aria-hidden="true">
            U
          </span>
          <span>USahapintar</span>
        </a>

        <p className={styles.eyebrow}>MULAI BANGUN KONEKSI</p>
        <h1 id="registration-title">Buat akun baru</h1>
        <p className={styles.description}>
          Pilih jenis akun dan daftarkan kontak yang akan digunakan untuk verifikasi.
        </p>

        <div className={styles.rolePicker} role="group" aria-label="Jenis akun">
          {roleOptions.map((option) => (
            <button
              className={role === option.value ? styles.roleActive : styles.roleOption}
              key={option.value}
              type="button"
              aria-pressed={role === option.value}
              onClick={() => setRole(option.value)}
            >
              <span className={styles.roleLabel}>{option.label}</span>
              <span className={styles.roleDescription}>{option.description}</span>
            </button>
          ))}
        </div>

        <form className={styles.form}>
          <label htmlFor="registration-display-name">Nama tampilan publik</label>
          <input
            id="registration-display-name"
            name="display_name"
            type="text"
            autoComplete="name"
            maxLength={120}
            placeholder="Nama yang akan terlihat oleh pengguna lain"
            required
            disabled
          />

          <fieldset className={styles.channelFieldset}>
            <legend>Kontak untuk verifikasi</legend>
            <div className={styles.channelPicker}>
              <button
                className={channel === "email" ? styles.channelActive : styles.channelOption}
                type="button"
                aria-pressed={channel === "email"}
                onClick={() => setChannel("email")}
              >
                Email
              </button>
              <button
                className={
                  channel === "whatsapp" ? styles.channelActive : styles.channelOption
                }
                type="button"
                aria-pressed={channel === "whatsapp"}
                onClick={() => setChannel("whatsapp")}
              >
                WhatsApp
              </button>
            </div>
          </fieldset>

          <label htmlFor="registration-contact">
            {channel === "email" ? "Alamat email" : "Nomor WhatsApp"}
          </label>
          <input
            id="registration-contact"
            name={channel === "email" ? "email" : "phone"}
            type={channel === "email" ? "email" : "tel"}
            autoComplete={channel === "email" ? "email" : "tel"}
            placeholder={
              channel === "email" ? "nama@email.com" : "Contoh: +6281234567890"
            }
            disabled
          />

          <label htmlFor="registration-password">Password</label>
          <input
            id="registration-password"
            name="password"
            type="password"
            autoComplete="new-password"
            maxLength={1024}
            disabled
          />

          <label htmlFor="registration-password-confirmation">
            Konfirmasi password
          </label>
          <input
            id="registration-password-confirmation"
            name="password_confirmation"
            type="password"
            autoComplete="new-password"
            maxLength={1024}
            disabled
          />

          <p className={styles.hint}>
            Setelah layanan registrasi aktif, kode OTP akan dikirim ke kontak yang
            dipilih. Akun baru dapat digunakan setelah kode berhasil diverifikasi.
          </p>
        </form>

        <div className={styles.notice} role="status">
          <strong>Pendaftaran belum dibuka.</strong>
          <span>
            Dokumen Ketentuan Layanan dan Kebijakan Privasi, kebijakan password, serta
            layanan verifikasi server masih dalam tahap finalisasi. Form ini merupakan
            rancangan antarmuka dan belum mengirim data.
          </span>
        </div>

        <button className={styles.submit} type="button" disabled>
          Pendaftaran segera tersedia
        </button>

        <p className={styles.loginPrompt}>
          Sudah punya akun? <a href="/login">Masuk</a>
        </p>
        <a className={styles.backLink} href="/">
          Kembali ke beranda
        </a>
      </section>
    </main>
  );
}
