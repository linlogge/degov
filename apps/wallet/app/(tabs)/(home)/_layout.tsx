import { useThemeColor } from "@/hooks/use-theme-color";
import { Stack } from "expo-router";
import { useColorScheme } from "react-native";

export default function HomeLayout() {
  const theme = useColorScheme();
  return (
    <Stack>
      <Stack.Screen name="index" />
      <Stack.Screen name="[id]" />
    </Stack>
  );
}
