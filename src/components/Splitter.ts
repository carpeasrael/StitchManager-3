export class Splitter {
  private el: HTMLElement;
  private property: string;
  private min: number;
  private max: number;
  private dragging = false;
  private startX = 0;
  private startValue = 0;
  private activeMoveHandler: ((ev: MouseEvent) => void) | null = null;
  private activeUpHandler: (() => void) | null = null;
  private readonly mouseDownHandler: (e: MouseEvent) => void;

  constructor(
    container: HTMLElement,
    property: string,
    min: number,
    max: number,
    defaultValue: number
  ) {
    this.property = property;
    this.min = min;
    this.max = max;

    this.el = document.createElement("div");
    this.el.className = "splitter";
    container.appendChild(this.el);

    // Set initial value if not already set
    const root = document.documentElement;
    if (!root.style.getPropertyValue(property)) {
      root.style.setProperty(property, `${defaultValue}px`);
    }

    this.mouseDownHandler = (e) => this.onMouseDown(e);
    this.el.addEventListener("mousedown", this.mouseDownHandler);
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
    this.dragging = false;
    this.el.removeEventListener("mousedown", this.mouseDownHandler);
    this.el.remove();
  }

  private onMouseDown(e: MouseEvent): void {
    if (this.dragging) return;
    e.preventDefault();
    this.dragging = true;
    this.startX = e.clientX;
    const current = document.documentElement.style.getPropertyValue(
      this.property
    );
    this.startValue = parseInt(current, 10) || 0;
    document.body.style.cursor = "col-resize";
    document.body.style.userSelect = "none";

    const onMouseMove = (ev: MouseEvent) => {
      if (!this.dragging) return;
      const delta = ev.clientX - this.startX;
      const newValue = Math.min(
        this.max,
        Math.max(this.min, this.startValue + delta)
      );
      document.documentElement.style.setProperty(
        this.property,
        `${newValue}px`
      );
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
