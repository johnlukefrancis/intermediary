// Path: app/src/app.tsx
// Description: Root component with tab state management and offline banner

import React, { useState } from "react";
import { TabBar } from "./components/tab_bar.js";
import { OfflineBanner } from "./components/offline_banner.js";
import { TexturePortalTab } from "./tabs/texture_portal_tab.js";
import { TriangleRainTab } from "./tabs/triangle_rain_tab.js";
import { IntermediaryTab } from "./tabs/intermediary_tab.js";
import type { TabId } from "./shared/protocol.js";

export function App(): React.JSX.Element {
  const [activeTab, setActiveTab] = useState<TabId>("intermediary");

  return (
    <div className="app">
      <OfflineBanner />
      <TabBar activeTab={activeTab} onTabChange={setActiveTab} />
      <main className="tab-content">
        {activeTab === "texture-portal" && <TexturePortalTab />}
        {activeTab === "triangle-rain" && <TriangleRainTab />}
        {activeTab === "intermediary" && <IntermediaryTab />}
      </main>
    </div>
  );
}
