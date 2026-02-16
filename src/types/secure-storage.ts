export interface SetSecretRequest {
  key: string;
  value: string;
  category: string;
  metadata?: Record<string, string>;
}

export interface GetSecretRequest {
  key: string;
  category: string;
}

export interface DeleteSecretRequest {
  key: string;
  category: string;
}

export interface ListKeysRequest {
  category: string;
}

export interface SecretResponse {
  key: string;
  value: string;
  category: string;
  metadata?: Record<string, string>;
}

export const SecretCategory = {
  API_KEY: "api_key",
  DATABASE_PASSWORD: "database_password",
  ENCRYPTION_KEY: "encryption_key",
  TOKEN: "token",
  CERTIFICATE: "certificate",
  CUSTOM: "custom",
} as const;

export type SecretCategoryType =
  (typeof SecretCategory)[keyof typeof SecretCategory];

export interface ApiKeyProvider {
  provider: string;
  hasKey: boolean;
}
