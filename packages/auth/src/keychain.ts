import keytar from "keytar";

const SERVICE = "oni-cli";
const ACCOUNT_API_KEY = "api-key";

export async function storeApiKey(key: string): Promise<void> {
  await keytar.setPassword(SERVICE, ACCOUNT_API_KEY, key);
}

export async function getApiKey(): Promise<string | null> {
  return keytar.getPassword(SERVICE, ACCOUNT_API_KEY);
}

export async function deleteApiKey(): Promise<boolean> {
  return keytar.deletePassword(SERVICE, ACCOUNT_API_KEY);
}

export async function hasApiKey(): Promise<boolean> {
  const key = await getApiKey();
  return key !== null && key.length > 0;
}
