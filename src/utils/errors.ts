/**
 * Audit Wave 5 (deferred from Wave 3 #6): central mapping from the Rust
 * `AppError.code` field to a human-readable German message. The backend
 * serialises every error as `{ code, message }` (see
 * `src-tauri/src/error.rs`). When the user sees the toast, prefer:
 *   1. The validation message (it's already authored for the user — short
 *      "Validierungsfehler: ..." string).
 *   2. A generic German message keyed by the code, with the raw message
 *      logged to the console for diagnostics.
 *   3. The caller's fallback string.
 */

interface BackendError {
  code?: string;
  message?: string;
}

const CODE_MESSAGES: Record<string, string> = {
  DATABASE: "Datenbankfehler — bitte erneut versuchen.",
  IO: "Dateisystemfehler — bitte Berechtigungen oder Speicherplatz prüfen.",
  PARSE: "Datei konnte nicht gelesen werden — Format prüfen.",
  AI: "KI-Anfrage fehlgeschlagen — Verbindung oder API-Schlüssel prüfen.",
  NOT_FOUND: "Datei oder Eintrag nicht gefunden.",
  VALIDATION: "Ungültige Eingabe.",
  INTERNAL: "Interner Fehler — bitte erneut versuchen.",
};

function asBackendError(e: unknown): BackendError | null {
  if (e && typeof e === "object") {
    const obj = e as BackendError;
    if (typeof obj.code === "string" || typeof obj.message === "string") {
      return obj;
    }
  }
  return null;
}

export function extractBackendMessage(e: unknown, fallback: string): string {
  const be = asBackendError(e);
  if (be) {
    // Validation messages are user-facing already — prefer them verbatim.
    if (be.code === "VALIDATION" && typeof be.message === "string" && be.message.trim()) {
      return be.message;
    }
    // For other codes log raw and return the friendly mapping.
    if (be.code && CODE_MESSAGES[be.code]) {
      if (be.message) {
        // eslint-disable-next-line no-console
        console.warn(`[${be.code}] ${be.message}`);
      }
      return CODE_MESSAGES[be.code];
    }
    if (typeof be.message === "string" && be.message.trim()) {
      return be.message;
    }
  }
  if (e instanceof Error && e.message) return e.message;
  return fallback;
}
