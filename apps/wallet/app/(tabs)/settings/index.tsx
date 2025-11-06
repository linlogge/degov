import { Image } from "expo-image";
import {
  AppRegistry,
  Button,
  Platform,
  ScrollView,
  StyleSheet,
  Text,
  View,
} from "react-native";

import { Collapsible } from "@/components/ui/collapsible";
import { ExternalLink } from "@/components/external-link";
import ParallaxScrollView from "@/components/parallax-scroll-view";
import { ThemedText } from "@/components/themed-text";
import { ThemedView } from "@/components/themed-view";
import { IconSymbol } from "@/components/ui/icon-symbol";
import { Fonts } from "@/constants/theme";
import { Stack } from "expo-router";
// import { add, uniffiInitAsync } from "identity-kit";

export default function TabTwoScreen() {
  return (
    <ScrollView>
      <Stack.Screen options={{ title: "Settings" }} />
      <View style={styles.container}>
        <Text>Setting1</Text>
        <Text>Setting2</Text>
        <Text>Setting3</Text>
        <Text>Setting4</Text>
        <Text>Setting5</Text>
      </View>
    </ScrollView>
  );
}

const styles = StyleSheet.create({
  container: {
    flex: 1,
    justifyContent: "center",
    alignItems: "center",
  },
});
