export type ClientRole = "business_owner" | "investor";

export type AdminSession = {
  user_id: string;
  role: "admin";
};

export type ClientSession = {
  user_id: string;
  role: ClientRole;
};

type ApiPayload = {
  message?: string;
  error?: string;
  user_id?: string;
  role?: string;
};

export class IdentityApiError extends Error {
  readonly status: number;

  constructor(status: number, message: string) {
    super(message);
    this.name = "IdentityApiError";
    this.status = status;
  }
}

async function requestIdentityApi(
  path: string,
  init: RequestInit = {},
): Promise<ApiPayload> {
  let response: Response;
  try {
    response = await fetch(path, {
      ...init,
      credentials: "same-origin",
      cache: "no-store",
      headers: {
        Accept: "application/json",
        ...(init.body ? { "Content-Type": "application/json" } : {}),
        ...init.headers,
      },
    });
  } catch {
    throw new IdentityApiError(
      0,
      "Koneksi belum tersedia. Periksa jaringan lalu coba lagi.",
    );
  }

  let payload: ApiPayload = {};
  if (response.status !== 204) {
    try {
      payload = (await response.json()) as ApiPayload;
    } catch {
      payload = {};
    }
  }

  if (!response.ok) {
    throw new IdentityApiError(
      response.status,
      payload.error ?? "Permintaan belum dapat diproses. Coba lagi.",
    );
  }

  return payload;
}

export async function loginClient(
  role: ClientRole,
  identifier: string,
  password: string,
): Promise<void> {
  const endpoint =
    role === "business_owner"
      ? "/api/v1/identity/login/owner"
      : "/api/v1/identity/login/investor";

  await requestIdentityApi(endpoint, {
    method: "POST",
    body: JSON.stringify({ identifier, password }),
  });
}

export async function getCurrentClient(
  expectedRole: ClientRole,
): Promise<ClientSession> {
  const payload = await requestIdentityApi("/api/v1/identity/me");
  if (
    typeof payload.user_id !== "string" ||
    payload.user_id.trim().length === 0 ||
    payload.role !== expectedRole
  ) {
    throw new IdentityApiError(
      502,
      "Sesi akun tidak sesuai dengan jenis akun yang dipilih.",
    );
  }

  return {
    user_id: payload.user_id,
    role: expectedRole,
  };
}

export async function beginAdminLogin(
  identifier: string,
  password: string,
): Promise<void> {
  await requestIdentityApi("/api/v1/identity/admin/login", {
    method: "POST",
    body: JSON.stringify({ identifier, password }),
  });
}

export async function completeAdminLogin(code: string): Promise<void> {
  await requestIdentityApi("/api/v1/identity/admin/login/totp", {
    method: "POST",
    body: JSON.stringify({ code }),
  });
}

export async function getCurrentAdmin(): Promise<AdminSession> {
  const payload = await requestIdentityApi("/api/v1/identity/admin/me");
  if (
    typeof payload.user_id !== "string" ||
    payload.user_id.trim().length === 0 ||
    payload.role !== "admin"
  ) {
    throw new IdentityApiError(502, "Respons sesi Admin tidak valid.");
  }
  return {
    user_id: payload.user_id,
    role: "admin",
  };
}

export async function logoutAdmin(): Promise<void> {
  await requestIdentityApi("/api/v1/identity/admin/logout", {
    method: "POST",
  });
}
