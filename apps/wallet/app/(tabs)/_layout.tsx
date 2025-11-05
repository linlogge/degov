import {
  Icon,
  Label,
  NativeTabs as Tabs,
} from "expo-router/unstable-native-tabs";
import React from "react";

import { HapticTab } from "@/components/haptic-tab";
import { IconSymbol } from "@/components/ui/icon-symbol";
import { Colors } from "@/constants/theme";
import { useColorScheme } from "@/hooks/use-color-scheme";
import { Stack } from "expo-router";

export default function TabLayout() {
  const colorScheme = useColorScheme();

  return (
    <Tabs>
      <Tabs.Trigger
        name="(home)"
        options={{
          title: "Wallet",
        }}
      >
        <Label>Wallet</Label>
        <Icon sf="wallet.bifold.fill" />
      </Tabs.Trigger>
      <Tabs.Trigger
        name="settings"
        options={{
          title: "Settings",
        }}
      >
        <Label>Settings</Label>
        <Icon sf="gearshape.fill" />
      </Tabs.Trigger>
      <Tabs.Trigger
        name="scan"
        options={{
          title: "Scan",
        }}
        role="search"
      >
        <Label>Scan</Label>
        <Icon sf="qrcode.viewfinder" />
      </Tabs.Trigger>
    </Tabs>
  );
}
