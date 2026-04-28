import crypto from "node:crypto";
import fs from "node:fs/promises";
import os from "node:os";
import path from "node:path";

interface SecretEnvelope {
  iv: string;
  tag: string;
  payload: string;
}

function deriveKey(): Buffer {
  const seed = `${os.hostname()}::${os.userInfo().username}::novelforge`;
  return crypto.scryptSync(seed, "novelforge-local-salt", 32);
}

function encrypt(value: string): SecretEnvelope {
  const iv = crypto.randomBytes(12);
  const key = deriveKey();
  const cipher = crypto.createCipheriv("aes-256-gcm", key, iv);
  const encrypted = Buffer.concat([cipher.update(value, "utf-8"), cipher.final()]);
  const tag = cipher.getAuthTag();
  return {
    iv: iv.toString("base64"),
    tag: tag.toString("base64"),
    payload: encrypted.toString("base64")
  };
}

function decrypt(envelope: SecretEnvelope): string {
  const key = deriveKey();
  const decipher = crypto.createDecipheriv(
    "aes-256-gcm",
    key,
    Buffer.from(envelope.iv, "base64")
  );
  decipher.setAuthTag(Buffer.from(envelope.tag, "base64"));
  const plain = Buffer.concat([
    decipher.update(Buffer.from(envelope.payload, "base64")),
    decipher.final()
  ]);
  return plain.toString("utf-8");
}

function secretPath(projectId: string): string {
  return path.join(os.homedir(), ".novelforge", "secrets", `${projectId}.json`);
}

export async function saveProjectApiKey(projectId: string, apiKey: string): Promise<void> {
  const filePath = secretPath(projectId);
  await fs.mkdir(path.dirname(filePath), { recursive: true });
  const envelope = encrypt(apiKey);
  await fs.writeFile(filePath, JSON.stringify(envelope, null, 2), "utf-8");
}

export async function loadProjectApiKey(projectId: string): Promise<string | undefined> {
  const filePath = secretPath(projectId);
  try {
    const raw = await fs.readFile(filePath, "utf-8");
    const envelope = JSON.parse(raw) as SecretEnvelope;
    return decrypt(envelope);
  } catch {
    return undefined;
  }
}
