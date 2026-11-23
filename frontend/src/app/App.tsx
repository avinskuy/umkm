import { useEffect, useState } from "react";
import { AdminDashboardPage } from "./AdminDashboardPage";
import { AdminLoginPage } from "./AdminLoginPage";
import { ResetPasswordPage } from "./ResetPasswordPage";
import styles from "./App.module.css";

export function App() {
  const [path, setPath] = useState(window.location.pathname);

  useEffect(() => {
    const updatePath = () => setPath(window.location.pathname);
    window.addEventListener("popstate", updatePath);
    return () => window.removeEventListener("popstate", updatePath);
  }, []);

  if (path === "/admin/login" || path === "/admin/login/") {
    return <AdminLoginPage />;
  }

  if (path === "/admin" || path === "/admin/") {
    return <AdminDashboardPage />;
  }

  if (path.endsWith("/reset-password") || path.endsWith("/reset-password/")) {
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
