const FONT_SIZE_MAP: Record<string, string> = {
  small: "12px",
  medium: "13px",
  large: "15px",
};

export function applyFontSize(size: string): void {
  document.documentElement.style.setProperty(
    "--font-size-body",
    FONT_SIZE_MAP[size] || FONT_SIZE_MAP.medium
  );
}
