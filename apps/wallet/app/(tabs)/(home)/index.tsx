import {
  Button,
  Pressable,
  ScrollView,
  StyleSheet,
  TouchableOpacity,
  View,
} from "react-native";
import MaterialIcons from "@expo/vector-icons/MaterialIcons";
import { Link, Stack, useRouter } from "expo-router";
import {
  Button as ButtonPrimitive,
  CircularProgress,
  Host,
  Image,
  Label,
  Text,
  VStack,
} from '@expo/ui/swift-ui';

import { ThemedText } from "@/components/themed-text";
import { ThemedView } from "@/components/themed-view";
import { useThemeColor } from "@/hooks/use-theme-color";
import { Logo } from "../../../../../packages/rn-ui/src/components/logo";
import { walletItems } from "../../../constants/wallet-items";
import { add } from "@dgv/react-native-identity";
import { IconSymbol } from "@/components/ui/icon-symbol";
import { HeaderButton } from "@react-navigation/elements";

export default function HomeScreen() {
  const router = useRouter();
  const backgroundColor = useThemeColor({}, "background");
  const cardBackground = useThemeColor(
    { light: "#FFFFFF", dark: "#1F2937" },
    "background",
  );
  const iconColor = useThemeColor({}, "icon");

  return (
    <>
      <Stack.Screen
        options={{
          title: "Wallet",
          headerLargeTitle: true,
          headerBlurEffect: "none",
          headerTransparent: true,
          unstable_headerRightItems: () => {
            return [
              {
                type: "button",
                icon: {
                  type: "sfSymbol",
                  name: "barcode.viewfinder"
                },
                label: "Add",
                onPress: () => {
                  router.navigate("/scan")
                }
              }
            ]
          }
        }}
      />
      <ScrollView
        style={[styles.container, { backgroundColor }]}
        contentContainerStyle={styles.contentContainer}
      >
        <ThemedView style={styles.header}>
          <ThemedText type="subtitle" style={styles.headerSubtitle}>
            {add(1, 88)} items
          </ThemedText>
        </ThemedView>

        <View style={styles.itemsContainer}>
          {walletItems.map((item) => (
            <TouchableOpacity
              key={item.id}
              style={[styles.card, { backgroundColor: cardBackground }]}
              activeOpacity={0.7}
              onPress={() => router.push(`/(tabs)/(home)/${item.id}`)}
            >
              <View
                style={[styles.iconContainer, {
                  backgroundColor: `${item.color}20`,
                }]}
              >
                <MaterialIcons name={item.icon} size={28} color={item.color} />
              </View>
              <View style={styles.cardContent}>
                <View style={styles.cardHeader}>
                  <ThemedText type="defaultSemiBold" style={styles.cardTitle}>
                    {item.title}
                  </ThemedText>
                  {item.verified && (
                    <MaterialIcons name="verified" size={20} color="#10B981" />
                  )}
                </View>
                <ThemedText type="default" style={styles.cardSubtitle}>
                  {item.subtitle}
                </ThemedText>
              </View>
              <MaterialIcons
                name="chevron-right"
                size={24}
                color={iconColor}
              />
            </TouchableOpacity>
          ))}
        </View>
      </ScrollView>
    </>
  );
}

const styles = StyleSheet.create({
  container: {
    flex: 1,
  },
  contentContainer: {
    padding: 16,
  },
  header: {
    marginBottom: 24,
  },
  headerTitle: {
    marginBottom: 4,
  },
  headerSubtitle: {
    fontSize: 16,
    opacity: 0.7,
  },
  itemsContainer: {
    gap: 12,
  },
  card: {
    flexDirection: "row",
    alignItems: "center",
    padding: 16,
    borderRadius: 12,
    marginBottom: 4,
    shadowColor: "#000",
    shadowOffset: { width: 0, height: 2 },
    shadowOpacity: 0.1,
    shadowRadius: 4,
    elevation: 2,
  },
  iconContainer: {
    width: 56,
    height: 56,
    borderRadius: 12,
    justifyContent: "center",
    alignItems: "center",
    marginRight: 16,
  },
  cardContent: {
    flex: 1,
  },
  cardHeader: {
    flexDirection: "row",
    alignItems: "center",
    gap: 8,
    marginBottom: 4,
  },
  cardTitle: {
    fontSize: 16,
  },
  cardSubtitle: {
    fontSize: 14,
    opacity: 0.7,
  },
});
