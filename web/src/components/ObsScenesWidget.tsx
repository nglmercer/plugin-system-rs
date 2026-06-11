import { h } from "preact";
import { useState, useEffect, useCallback } from "preact/hooks";
import { fetchObsScenes, setCurrentScene, fetchObsTransitions, setTransition, fetchObsSceneItems, setSceneItemEnabled } from "../lib/api";

interface ObsScene {
  name: string;
  index: number;
}

interface ObsTransition {
  name: string;
  kind: string;
  duration: number;
}

interface ObsSceneItem {
  id: number;
  name: string;
  enabled: boolean;
}

export function ObsScenesWidget({ settings }: { settings: Record<string, any> }) {
  const [scenes, setScenes] = useState<ObsScene[]>([]);
  const [currentScene, setCurrentSceneState] = useState("");
  const [transitions, setTransitions] = useState<ObsTransition[]>([]);
  const [sceneItems, setSceneItems] = useState<ObsSceneItem[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const variant = (settings.variant || "compact") as string;

  const fetchData = useCallback(async () => {
    try {
      const scenesData = await fetchObsScenes();
      if (scenesData) {
        setScenes(scenesData.scenes || []);
        setCurrentSceneState(scenesData.current_scene || "");
      }
      const transData = await fetchObsTransitions();
      if (transData) setTransitions(transData);
      setError(null);
    } catch {
      setError("Failed to fetch scenes");
    }
    setLoading(false);
  }, []);

  useEffect(() => {
    fetchData();
    const interval = setInterval(fetchData, settings.refreshInterval || 2000);
    return () => clearInterval(interval);
  }, [fetchData, settings.refreshInterval]);

  useEffect(() => {
    if (variant === "detailed" && currentScene) {
      fetchObsSceneItems(currentScene).then((items) => {
        if (items) setSceneItems(items);
      }).catch(() => {});
    }
  }, [variant, currentScene]);

  async function handleSceneClick(sceneName: string) {
    try {
      await setCurrentScene(sceneName);
      setCurrentSceneState(sceneName);
    } catch {}
  }

  async function handleTransitionClick(name: string) {
    try {
      await setTransition(name);
    } catch {}
  }

  async function handleSceneItemToggle(itemId: number, enabled: boolean) {
    if (!currentScene) return;
    try {
      await setSceneItemEnabled(currentScene, itemId, !enabled);
      setSceneItems((prev) =>
        prev.map((item) =>
          item.id === itemId ? { ...item, enabled: !enabled } : item
        )
      );
    } catch {}
  }

  if (loading) return h("div", { class: "obsscene-loading" }, "Loading...");
  if (error) return h("div", { class: "obsscene-error" }, error);

  if (variant === "minimal") {
    return h("div", { class: "obsscene-variant minimal" },
      h("div", { class: "obsscene-current" }, currentScene || "No scene"),
      h("div", { class: "obsscene-grid-mini" },
        scenes.map((scene) =>
          h("button", {
            key: scene.name,
            class: `obsscene-mini-btn ${scene.name === currentScene ? "active" : ""}`,
            onClick: () => handleSceneClick(scene.name),
          }, scene.name.length > 8 ? scene.name.substring(0, 8) + ".." : scene.name)
        ),
      ),
    );
  }

  if (variant === "detailed") {
    return h("div", { class: "obsscene-variant detailed" },
      h("div", { class: "obsscene-header" },
        h("span", { class: "obsscene-title" }, "Scenes"),
        h("span", { class: "obsscene-current-label" }, currentScene || "None"),
      ),
      h("div", { class: "obsscene-list" },
        scenes.map((scene) =>
          h("button", {
            key: scene.name,
            class: `obsscene-item-btn ${scene.name === currentScene ? "active" : ""}`,
            onClick: () => handleSceneClick(scene.name),
          },
            h("span", { class: "obsscene-item-name" }, scene.name),
            h("span", { class: "obsscene-item-index" }, `#${scene.index}`),
          )
        ),
      ),
      transitions.length > 0 && h("div", { class: "obsscene-section" },
        h("span", { class: "obsscene-section-title" }, "Transitions"),
        h("div", { class: "obsscene-transition-list" },
          transitions.map((t) =>
            h("button", {
              key: t.name,
              class: "obsscene-transition-btn",
              onClick: () => handleTransitionClick(t.name),
            }, t.name)
          ),
        ),
      ),
      sceneItems.length > 0 && h("div", { class: "obsscene-section" },
        h("span", { class: "obsscene-section-title" }, `Sources in "${currentScene}"`),
        h("div", { class: "obsscene-items-list" },
          sceneItems.map((item) =>
            h("div", { key: item.id, class: "obsscene-source-item" },
              h("button", {
                class: `obsscene-eye-btn ${item.enabled ? "active" : ""}`,
                onClick: () => handleSceneItemToggle(item.id, item.enabled),
              }, item.enabled ? "ON" : "OFF"),
              h("span", { class: "obsscene-source-name" }, item.name),
            )
          ),
        ),
      ),
    );
  }

  return h("div", { class: "obsscene-variant compact" },
    h("div", { class: "obsscene-header" },
      h("span", { class: "obsscene-title" }, "Scenes"),
      h("span", { class: "obsscene-current-label" }, currentScene || "None"),
    ),
    h("div", { class: "obsscene-list" },
      scenes.map((scene) =>
        h("button", {
          key: scene.name,
          class: `obsscene-item-btn ${scene.name === currentScene ? "active" : ""}`,
          onClick: () => handleSceneClick(scene.name),
        }, scene.name)
      ),
    ),
  );
}
