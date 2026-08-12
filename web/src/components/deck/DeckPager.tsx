import { h } from "preact";

/**
 * Page navigation, and — while a widget is being dragged — the drop target
 * for moving it between pages.
 *
 * Making the dots themselves the drop zone is what lets a widget cross pages
 * without a modal or a menu: the user is already dragging, and the pager is
 * already the thing that means "page". A dot enlarges when the pointer is over
 * it during a drag, so the target is visible before releasing.
 */

export interface DeckPagerProps {
  pages: number;
  page: number;
  onPageChange: (page: number) => void;
  /** True while a widget drag is in flight, which turns dots into targets. */
  dragging?: boolean;
  /** Page the pointer is currently over during a drag, if any. */
  dropPage?: number | null;
  /** Add a page. Absent outside arrange mode. */
  onAddPage?: () => void;
}

export function DeckPager(props: DeckPagerProps) {
  const { pages, page, onPageChange, dragging, dropPage, onAddPage } = props;

  return h(
    "div",
    { class: `deck-pager${dragging ? " is-drop-target" : ""}` },
    h(
      "button",
      {
        class: "deck-pager-arrow",
        disabled: page === 0,
        onClick: () => onPageChange(Math.max(0, page - 1)),
        "aria-label": "Previous page",
      },
      "‹",
    ),
    h(
      "div",
      { class: "deck-dots" },
      Array.from({ length: pages }, (_, i) =>
        h(
          "button",
          {
            key: `dot-${i}`,
            // `data-page` is how the drag handler identifies which dot the
            // pointer is over; hit-testing DOM beats tracking dot geometry
            // separately and getting the two out of sync.
            "data-deck-page": String(i),
            class: [
              "deck-dot",
              i === page ? "is-active" : "",
              dragging && dropPage === i ? "is-drop-hover" : "",
            ]
              .filter(Boolean)
              .join(" "),
            onClick: () => onPageChange(i),
            "aria-label": `Page ${i + 1}`,
            "aria-current": i === page ? "true" : undefined,
          },
          h("span", { class: "deck-dot-label" }, String(i + 1)),
        ),
      ),
      // A "new page" target only while arranging, so a drag has somewhere to
      // go when every existing page is full.
      onAddPage &&
        h(
          "button",
          {
            "data-deck-page": String(pages),
            class: `deck-dot deck-dot-add${
              dragging && dropPage === pages ? " is-drop-hover" : ""
            }`,
            onClick: onAddPage,
            title: "Add page",
            "aria-label": "Add page",
          },
          h("span", { class: "deck-dot-label" }, "+"),
        ),
    ),
    h(
      "button",
      {
        class: "deck-pager-arrow",
        disabled: page >= pages - 1,
        onClick: () => onPageChange(Math.min(pages - 1, page + 1)),
        "aria-label": "Next page",
      },
      "›",
    ),
  );
}
