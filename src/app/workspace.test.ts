import { describe, expect, it } from "vitest";

import { normalizeWorkspace } from "./workspace";

describe("normalizeWorkspace", () => {
  it("restores only valid workspace values and always provides Home", () => {
    const workspace = normalizeWorkspace({
      activeId: "server-prod",
      selectedId: "prod",
      tabs: [
        { id: "server-prod", title: "Production", profileId: "prod" },
      ],
      browsers: {
        prod: {
          directoryPath: "/var/log/service",
          dockHeight: 420,
          showTerminalDock: true,
          view: "files",
        },
        invalid: { directoryPath: "/tmp" },
      },
    });

    expect(workspace.tabs.map((tab) => tab.id)).toEqual(["home", "server-prod"]);
    expect(workspace.activeId).toBe("server-prod");
    expect(workspace.browsers).toEqual({
      prod: {
        directoryPath: "/var/log/service",
        dockHeight: 420,
        showTerminalDock: true,
        view: "files",
      },
    });
  });

  it("falls back safely when persisted data is malformed", () => {
    expect(normalizeWorkspace({ activeId: "missing", tabs: [null] })).toEqual({
      activeId: "home",
      browsers: {},
      selectedId: "",
      tabs: [{ id: "home", title: "Home" }],
    });
  });
});
