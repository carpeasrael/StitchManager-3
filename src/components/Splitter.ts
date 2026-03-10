export class Splitter {
  private el: HTMLElement;
  private property: string;
  private min: number;
  private max: number;
  private dragging = false;
  private startX = 0;
  private startValue = 0;

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

    this.el.addEventListener("mousedown", (e) => this.onMouseDown(e));
  }

  private onMouseDown(e: MouseEvent): void {
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
      document.body.style.cursor = "";
      document.body.style.userSelect = "";
      document.removeEventListener("mousemove", onMouseMove);
      document.removeEventListener("mouseup", onMouseUp);
    };

    document.addEventListener("mousemove", onMouseMove);
    document.addEventListener("mouseup", onMouseUp);
  }
}
