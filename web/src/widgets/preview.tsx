import { h, VNode, ComponentChildren } from "preact";

/**
 * Building blocks for the miniatures in the wizard's style step.
 *
 * They exist so a variant preview is a line or two inside the definition that
 * owns it. When previews lived in one big `switch`, adding a variant without a
 * preview was easy and invisible — it fell through to a generic "Action" box.
 */

export function box(kind: string, ...children: ComponentChildren[]): VNode<any> {
  return h("div", { class: `variant-preview ${kind}` }, ...children);
}

export function line(...children: ComponentChildren[]): VNode<any> {
  return h("div", { class: "mini-row" }, ...children);
}

export function bar(percent: number, color = "var(--sd-success)"): VNode<any> {
  return h(
    "div",
    { class: "mini-bar" },
    h("div", {
      class: "mini-bar-fill",
      style: { width: `${percent}%`, background: color },
    }),
  );
}

export function chip(label: string, active = false): VNode<any> {
  return h("div", { class: `mini-btn ${active ? "active" : ""}` }, label);
}

export function chips(...labels: (string | [string, boolean])[]): VNode<any> {
  return h(
    "div",
    { class: "mini-btns" },
    labels.map((entry) =>
      Array.isArray(entry) ? chip(entry[0], entry[1]) : chip(entry),
    ),
  );
}

export function dot(color: "green" | "red"): VNode<any> {
  return h("div", { class: `mini-dot ${color}` });
}

export function text(value: string, kind?: string): VNode<any> {
  return h("div", { class: kind }, value);
}
