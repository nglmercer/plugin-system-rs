import { h } from "preact";
import { useEffect, useState } from "preact/hooks";
import { QrButton } from "./QrModal";
import { Icons } from "../ui/icons/Icons";
import { useTheme } from "../ui";
import { t, getLocale, setLocale, getAvailableLocales } from "../lib/i18n";
import "./FabMenu.css";

export type Page = "dashboard" | "profiles" | "plugins";

/**
 * The floating menu: navigation, dashboard actions, and the utility row.
 *
 * Everything that used to live in a dashboard header now lives here — the
 * add-widget button, and the arrange toggle. The header had room for exactly
 * one of them on a phone, and it chose neither (it was `display: none`).
 */
export interface FabMenuProps {
  page: Page;
  onNavigate: (page: Page) => void;
  onAddWidget: () => void;
  arranging: boolean;
  onToggleArrange: () => void;
}

export function FabMenu({ page, onNavigate, onAddWidget, arranging, onToggleArrange }: FabMenuProps) {
  const themeApi = useTheme();
  const theme = themeApi.resolved;
  const [open, setOpen] = useState(false);
  const [locale, setLocaleState] = useState(getLocale());

  // The drawer must not scroll the page behind it while it is open.
  useEffect(() => {
    document.body.style.overflow = open ? "hidden" : "";
    return () => {
      document.body.style.overflow = "";
    };
  }, [open]);

  function navigateTo(target: Page) {
    setOpen(false);
    onNavigate(target);
  }

  function act(fn: () => void) {
    setOpen(false);
    fn();
  }

  async function handleLocaleChange(newLocale: string) {
    await setLocale(newLocale);
    setLocaleState(newLocale);
  }

  const onDashboard = page === "dashboard";

  return h(
    "div",
    null,
    open && h("div", { class: "fab-overlay", onClick: () => setOpen(false) }),
    h(
      "nav",
      { class: `fab-menu${open ? " open" : ""}`, "aria-hidden": !open },
      h(NavItem, {
        active: page === "dashboard",
        icon: Icons.dashboard,
        label: t("nav.dashboard"),
        onClick: () => navigateTo("dashboard"),
      }),
      h(NavItem, {
        active: page === "profiles",
        icon: Icons.profiles,
        label: t("nav.profiles"),
        onClick: () => navigateTo("profiles"),
      }),
      h(NavItem, {
        active: page === "plugins",
        icon: Icons.plugins,
        label: t("nav.plugins"),
        onClick: () => navigateTo("plugins"),
      }),

      onDashboard &&
        h("div", { class: "fab-section" },
          h("button", {
            class: "fab-action fab-add",
            type: "button",
            onClick: () => act(onAddWidget),
          }, h(Icons.plus, null), t("dashboard.addWidget")),
          h("button", {
            class: `fab-action${arranging ? " is-active" : ""}`,
            type: "button",
            onClick: () => act(onToggleArrange),
          }, h(Icons.edit, null), arranging ? t("dashboard.done") : t("dashboard.edit")),
        ),

      h(
        "div",
        { class: "fab-footer" },
        h("div", { class: "fab-footer-row" },
          h(
            "button",
            {
              class: "fab-util",
              type: "button",
              onClick: () => themeApi.toggle(),
              title: t("theme.switchTo", { theme: theme === "dark" ? t("theme.light") : t("theme.dark") }),
              "aria-label": t("theme.switchTo", { theme: theme === "dark" ? t("theme.light") : t("theme.dark") }),
            },
            theme === "dark" ? h(Icons.sun, null) : h(Icons.moon, null),
          ),
          h(
            "select",
            {
              class: "fab-lang",
              value: locale,
              onChange: (e: Event) => handleLocaleChange((e.target as HTMLSelectElement).value),
              "aria-label": "Language",
            },
            getAvailableLocales().map((loc) => h("option", { key: loc, value: loc }, loc.toUpperCase())),
          ),
          h(QrButton, null),
        ),
      ),
    ),
    h(
      "button",
      {
        class: `fab-burger${open ? " open" : ""}`,
        type: "button",
        onClick: () => setOpen(!open),
        "aria-label": "Menu",
        "aria-expanded": open,
      },
      h("span", null),
      h("span", null),
      h("span", null),
    ),
  );
}

function NavItem({
  active,
  icon,
  label,
  onClick,
}: {
  active: boolean;
  icon: () => h.JSX.Element;
  label: string;
  onClick: () => void;
}) {
  return h(
    "button",
    {
      class: `fab-nav${active ? " active" : ""}`,
      type: "button",
      onClick,
      "aria-current": active ? "page" : undefined,
    },
    h(icon, null),
    label,
  );
}
