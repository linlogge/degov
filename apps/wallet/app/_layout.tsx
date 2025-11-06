import {
  DarkTheme,
  DefaultTheme,
  ThemeProvider,
} from "@react-navigation/native";
import { Stack } from "expo-router";
import { useColorScheme } from "@/hooks/use-color-scheme";
import { StatusBar } from "expo-status-bar";
import { uniffiInitAsync } from "@dgv/react-native-identity";
import { useEffect, useState } from "react";
import { Text } from "react-native";

export default function Layout() {
  const colorScheme = useColorScheme();
  const [loaded, setLoaded] = useState<boolean>(false);

  useEffect(() => {
    async function load() {
      await uniffiInitAsync();
      setLoaded(true);
    }

    load()
  }, [])

  if (!loaded) return (<Text>Loading</Text>)

  return (
    <ThemeProvider value={colorScheme === "dark" ? DarkTheme : DefaultTheme}>
      <Stack initialRouteName="scan">
        <Stack.Screen
          name="scan"
          options={{ presentation: "modal", title: "Scan" }}
        />
        <Stack.Screen name="(tabs)" options={{ headerShown: false }} />
      </Stack>
      <StatusBar style="auto" />
    </ThemeProvider>
  );
}
