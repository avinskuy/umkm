import { useEffect, useState } from "react";
import {
  getCurrentAdmin,
  IdentityApiError,
  logoutAdmin,
  type AdminSession,
} from "./identityApi";
import { navigateTo } from "./navigation";
import styles from "./AdminDashboardPage.module.css";

export function AdminDashboardPage() {
  const [admin, setAdmin] = useState<AdminSession | null>(null);
  const [loading, setLoading] = useState(true);
  const [pendingLogout, setPendingLogout] = useState(false);
  const [notice, setNotice] = useState("");
  const [retryCount, setRetryCount] = useState(0);

  useEffect(() => {
    let active = true;

    async function loadSession() {
      setLoading(true);
      setNotice("");
      try {
        const session = await getCurrentAdmin();
        if (active) {
          setAdmin(session);
        }
      } catch (error) {
        if (!active) {
          return;
        }
        if (error instanceof IdentityApiError && error.status === 401) {
          navigateTo("/admin/login");
          return;
        }
        setNotice(
          error instanceof IdentityApiError
            ? error.message
            : "Sesi Admin belum dapat diperiksa. Coba lagi.",
        );
      } finally {
        if (active) {
          setLoading(false);
        }
      }
    }

    void loadSession();
    return () => {
      active = false;
    };
  }, [retryCount]);

  async function logout() {
    setPendingLogout(true);
    setNotice("");
    try {
      await logoutAdmin();
      navigateTo("/admin/login");
    } catch (error) {
      setNotice(
        error instanceof IdentityApiError
          ? error.message
          : "Logout belum berhasil. Coba lagi.",
      );
    } finally {
      setPendingLogout(false);
    }
  }

  if (loading) {
    return (
      <main className={styles.loadingPage} aria-busy="true">
        <p className={styles.eyebrow}>USAHAPINTAR · ADMIN</p>
        <h1>Memeriksa sesi administrator</h1>
        <p>Mohon tunggu sebentar.</p>
      </main>
    );
  }

  if (!admin) {
    return (
      <main className={styles.loadingPage}>
        <p className={styles.eyebrow}>USAHAPINTAR · ADMIN</p>
        <h1>Sesi belum tersedia</h1>
        <p className={styles.error} role="alert">{notice}</p>
        <button
          type="button"
          onClick={() => setRetryCount((value) => value + 1)}
        >
          Coba lagi
        </button>
        <button
          className={styles.secondaryButton}
          type="button"
          onClick={() => navigateTo("/admin/login")}
        >
          Kembali ke login
        </button>
      </main>
    );
  }

  return (
    <main className={styles.page}>
      <header className={styles.topbar}>
        <a className={styles.brand} href="/" aria-label="USahapintar beranda">
          <span className={styles.brandMark} aria-hidden="true">U</span>
          <span>USahapintar</span>
          <span className={styles.adminLabel}>ADMIN</span>
        </a>
        <button type="button" onClick={logout} disabled={pendingLogout}>
          {pendingLogout ? "Keluar..." : "Logout"}
        </button>
      </header>

      <section
        className={styles.content}
        aria-labelledby="admin-dashboard-title"
      >
        <p className={styles.eyebrow}>ADMINISTRATOR WORKSPACE</p>
        <h1 id="admin-dashboard-title">Dashboard Admin</h1>
        <p className={styles.description}>
          Sesi administrator terverifikasi. Area operasional akan diaktifkan
          secara bertahap setelah modul dan kontrol aksesnya selesai diaudit.
        </p>

        <section
          className={styles.sessionCard}
          aria-label="Informasi sesi Admin"
        >
          <div>
            <span className={styles.fieldLabel}>Status sesi</span>
            <strong className={styles.sessionStatus}>
              Aktif dan terverifikasi
            </strong>
          </div>
          <div>
            <span className={styles.fieldLabel}>Peran</span>
            <strong>{admin.role}</strong>
          </div>
          <div>
            <span className={styles.fieldLabel}>Admin user ID</span>
            <code>{admin.user_id}</code>
          </div>
        </section>

        <div className={styles.notice} role="status">
          Fondasi autentikasi Admin sudah aktif. Fitur pengelolaan belum tersedia
          pada tahap ini.
        </div>
        {notice && <p className={styles.error} role="alert">{notice}</p>}
      </section>
    </main>
  );
}
