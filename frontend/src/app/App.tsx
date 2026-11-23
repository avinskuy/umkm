import { ResetPasswordPage } from "./ResetPasswordPage";
import styles from "./App.module.css";

export function App() {
  if (window.location.pathname.replace(/\\/$/, "").endsWith("/reset-password")) {
    return <ResetPasswordPage />;
  }

  return (
    <main className={styles.shell}>
      <div className={styles.brandMark} aria-hidden="true">
        U
      </div>
      <p className={styles.eyebrow}>BUSINESS NETWORKING PLATFORM</p>
      <h1>USahapintar</h1>
      <p className={styles.description}>
        Connecting business owners and investors through meaningful opportunities.
      </p>
      <span className={styles.status}>Project foundation is ready to grow.</span>
    </main>
  );
}
