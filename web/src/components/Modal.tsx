import { h, ComponentChildren } from "preact";

/**
 * The shared modal shell: a click-to-close overlay and a card that stops the
 * click. QrModal, the widget library and the wizard all used to carry private
 * copies of this pair; they now share one, and a phone-sized screen gets the
 * bottom-sheet treatment with `sheet`.
 */
export function Modal({
  onClose,
  class: cls,
  sheet,
  children,
}: {
  onClose: () => void;
  class?: string;
  /** Slide up from the bottom edge on phones instead of centering. */
  sheet?: boolean;
  children?: ComponentChildren;
}) {
  return h(
    "div",
    { class: "modal-overlay", onClick: onClose },
    h(
      "div",
      {
        class: ["modal-card", sheet ? "sheet" : "", cls ?? ""].filter(Boolean).join(" "),
        role: "dialog",
        "aria-modal": "true",
        onClick: (e: Event) => e.stopPropagation(),
      },
      children,
    ),
  );
}

/** The ✕ button every modal's header carries. */
export function ModalClose({ onClose, label }: { onClose: () => void; label: string }) {
  return h(
    "button",
    { class: "modal-close", type: "button", onClick: onClose, "aria-label": label },
    "✕",
  );
}
