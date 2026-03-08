export abstract class Component {
  protected el: HTMLElement;
  private subscriptions: Array<() => void> = [];

  constructor(container: HTMLElement) {
    this.el = container;
  }

  abstract render(): void;

  protected subscribe(unsubscribe: () => void): void {
    this.subscriptions.push(unsubscribe);
  }

  destroy(): void {
    this.subscriptions.forEach((unsub) => unsub());
    this.subscriptions = [];
    this.el.innerHTML = "";
  }
}
