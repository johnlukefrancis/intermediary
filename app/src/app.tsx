// Path: app/src/app.tsx
// Description: Root component with tab state management

import React, { useState } from "react";
import { TabBar } from "./components/tab_bar";
import { TexturePortalTab } from "./tabs/texture_portal_tab";
import { TriangleRainTab } from "./tabs/triangle_rain_tab";
import { IntermediaryTab } from "./tabs/intermediary_tab";
import type { TabId } from "./shared/protocol";

export function App(): React.JSX.Element {
  const [activeTab, setActiveTab] = useState<TabId>("intermediary");

  return (
    <div className="app">
      <TabBar activeTab={activeTab} onTabChange={setActiveTab} />
      <main className="tab-content">
        {activeTab === "texture-portal" && <TexturePortalTab />}
        {activeTab === "triangle-rain" && <TriangleRainTab />}
        {activeTab === "intermediary" && <IntermediaryTab />}
      </main>
    </div>
  );
}
