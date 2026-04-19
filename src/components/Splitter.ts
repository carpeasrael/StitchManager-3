import * as SettingsService from "../services/SettingsService";

/**
 * Audit Wave 3 usability:
 * - role="separator" + aria-orientation/valuemin/max/now so assistive tech
 *   exposes the splitter as a proper resize control.
 * - tabindex=0 + Arrow/Home/End keyboard handlers (16 px nudge by default,
 *   shift = 64 px coarse step) so keyboard-only users can resize.
 * - Resized width is persisted to the settings DB on `mouseup`/Arrow-release,
 *   then restored on next launch (settings key derived from the CSS
 *   property name).
 */
export class Splitter {
  private el: HTMLElement;
  private property: string;
  private settingKey: string;
  private min: number;
  private max: number;
  private dragging = false;
  private startX = 0;
  private startValue = 0;
  private persistTimer: ReturnType<typeof setTimeout> | null = null;
  private activeMoveHandler: ((ev: MouseEvent) => void) | null = null;
  private activeUpHandler: (() => void) | null = null;
  private readonly mouseDownHandler: (e: MouseEvent) => void;
  private readonly keyDownHandler: (e: KeyboardEvent) => void;

  constructor(
    container: HTMLElement,
    property: string,
    min: number,
    max: number,
    defaultValue: number,
    label = "Spaltenbreite anpassen"
  ) {
    this.property = property;
    this.min = min;
    this.max = max;
    // Map "--sidebar-width" → "splitter:sidebar-width" for the settings key.
    this.settingKey = `splitter:${property.replace(/^--/, "")}`;

    this.el = document.createElement("div");
    this.el.className = "splitter";
    this.el.setAttribute("role", "separator");
    this.el.setAttribute("aria-orientation", "vertical");
    this.el.setAttribute("aria-valuemin", String(min));
    this.el.setAttribute("aria-valuemax", String(max));
    this.el.setAttribute("aria-label", label);
    this.el.tabIndex = 0;
    container.appendChild(this.el);

    // Restore persisted value (async — falls back to defaultValue while loading).
    const root = document.documentElement;
    if (!root.style.getPropertyValue(property)) {
      root.style.setProperty(property, `${defaultValue}px`);
      this.updateAriaValue(defaultValue);
    } else {
      this.updateAriaValue(this.currentValue());
    }
    SettingsService.getSetting(this.settingKey)
      .then((raw) => {
        const parsed = parseInt(raw ?? "", 10);
        if (Number.isFinite(parsed) && parsed >= min && parsed <= max) {
          root.style.setProperty(property, `${parsed}px`);
          this.updateAriaValue(parsed);
        }
      })
      .catch(() => { /* setting may not exist yet; keep default */ });

    this.mouseDownHandler = (e) => this.onMouseDown(e);
    this.keyDownHandler = (e) => this.onKeyDown(e);
    this.el.addEventListener("mousedown", this.mouseDownHandler);
    this.el.addEventListener("keydown", this.keyDownHandler);
  }

  destroy(): void {
    if (this.activeMoveHandler) {
      document.removeEventListener("mousemove", this.activeMoveHandler);
    }
    if (this.activeUpHandler) {
      document.removeEventListener("mouseup", this.activeUpHandler);
    }
    if (this.dragging) {
      document.body.style.cursor = "";
      document.body.style.userSelect = "";
    }
    if (this.persistTimer) {
      clearTimeout(this.persistTimer);
      this.persistTimer = null;
    }
    this.dragging = false;
    this.el.removeEventListener("mousedown", this.mouseDownHandler);
    this.el.removeEventListener("keydown", this.keyDownHandler);
    this.el.remove();
  }

  private currentValue(): number {
    const raw = document.documentElement.style.getPropertyValue(this.property);
    return parseInt(raw, 10) || 0;
  }

  private setValue(newValue: number): void {
    const clamped = Math.min(this.max, Math.max(this.min, newValue));
    document.documentElement.style.setProperty(this.property, `${clamped}px`);
    this.updateAriaValue(clamped);
    this.schedulePersist(clamped);
  }

  private updateAriaValue(v: number): void {
    this.el.setAttribute("aria-valuenow", String(v));
  }

  private schedulePersist(value: number): void {
    if (this.persistTimer) clearTimeout(this.persistTimer);
    this.persistTimer = setTimeout(() => {
      this.persistTimer = null;
      SettingsService.setSetting(this.settingKey, String(value)).catch(() => {
        /* best-effort persistence */
      });
    }, 250);
  }

  private onKeyDown(e: KeyboardEvent): void {
    const step = e.shiftKey ? 64 : 16;
    let next: number | null = null;
    switch (e.key) {
      case "ArrowLeft":
        next = this.currentValue() - step;
        break;
      case "ArrowRight":
        next = this.currentValue() + step;
        break;
      case "Home":
        next = this.min;
        break;
      case "End":
        next = this.max;
        break;
      default:
        return;
    }
    e.preventDefault();
    this.setValue(next);
  }

  private onMouseDown(e: MouseEvent): void {
    if (this.dragging) return;
    e.preventDefault();
    this.dragging = true;
    this.startX = e.clientX;
    this.startValue = this.currentValue();
    document.body.style.cursor = "col-resize";
    document.body.style.userSelect = "none";

    const onMouseMove = (ev: MouseEvent) => {
      if (!this.dragging) return;
      const delta = ev.clientX - this.startX;
      this.setValue(this.startValue + delta);
    };

    const onMouseUp = () => {
      this.dragging = false;
      this.activeMoveHandler = null;
      this.activeUpHandler = null;
      document.body.style.cursor = "";
      document.body.style.userSelect = "";
      document.removeEventListener("mousemove", onMouseMove);
      document.removeEventListener("mouseup", onMouseUp);
    };

    this.activeMoveHandler = onMouseMove;
    this.activeUpHandler = onMouseUp;
    document.addEventListener("mousemove", onMouseMove);
    document.addEventListener("mouseup", onMouseUp);
  }
}
