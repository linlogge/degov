import { ScrollView, StyleSheet, View } from "react-native";
import MaterialIcons from "@expo/vector-icons/MaterialIcons";
import { useLocalSearchParams, useRouter, Stack } from "expo-router";

import { ThemedText } from "@/components/themed-text";
import { ThemedView } from "@/components/themed-view";
import { useThemeColor } from "@/hooks/use-theme-color";
import { getWalletItemById } from "./wallet-items";

export default function WalletItemDetail() {
  const { id } = useLocalSearchParams<{ id: string }>();
  const router = useRouter();
  const backgroundColor = useThemeColor({}, "background");
  const cardBackground = useThemeColor(
    { light: "#FFFFFF", dark: "#1F2937" },
    "background",
  );
  const borderColor = useThemeColor({ light: "#E5E7EB", dark: "#374151" }, "background");

  const item = getWalletItemById(id || "");

  if (!item) {
    return (
      <>
        <Stack.Screen options={{ title: "Not Found" }} />
        <ThemedView style={styles.container}>
          <ThemedText type="title">Item not found</ThemedText>
        </ThemedView>
      </>
    );
  }

  return (
    <>
      <Stack.Screen
        options={{
          title: item.title,
          headerBackTitle: "Wallet",
        }}
      />
      <ScrollView
        style={[styles.container, { backgroundColor }]}
        contentContainerStyle={styles.contentContainer}
      >
        {/* Header Card */}
        <ThemedView
          style={[styles.headerCard, { backgroundColor: cardBackground }]}
        >
          <View
            style={[
              styles.iconContainer,
              { backgroundColor: `${item.color}20` },
            ]}
          >
            <MaterialIcons name={item.icon} size={48} color={item.color} />
          </View>
          <View style={styles.headerContent}>
            <View style={styles.titleRow}>
              <ThemedText type="title" style={styles.title}>
                {item.title}
              </ThemedText>
              {item.verified && (
                <MaterialIcons name="verified" size={24} color="#10B981" />
              )}
            </View>
            <ThemedText type="subtitle" style={styles.subtitle}>
              {item.subtitle}
            </ThemedText>
          </View>
        </ThemedView>

        {/* Details Section */}
        <ThemedView style={styles.section}>
          <ThemedText type="defaultSemiBold" style={styles.sectionTitle}>
            Details
          </ThemedText>

          {item.description && (
            <View style={[styles.detailRow, { borderBottomColor: borderColor }]}>
              <ThemedText type="default" style={styles.detailLabel}>
                Description
              </ThemedText>
              <ThemedText type="default" style={styles.detailValue}>
                {item.description}
              </ThemedText>
            </View>
          )}

          {item.issuer && (
            <View style={[styles.detailRow, { borderBottomColor: borderColor }]}>
              <ThemedText type="default" style={styles.detailLabel}>
                Issuer
              </ThemedText>
              <ThemedText type="defaultSemiBold" style={styles.detailValue}>
                {item.issuer}
              </ThemedText>
            </View>
          )}

          {item.category && (
            <View style={[styles.detailRow, { borderBottomColor: borderColor }]}>
              <ThemedText type="default" style={styles.detailLabel}>
                Category
              </ThemedText>
              <View style={[styles.categoryBadge, { backgroundColor: `${item.color}20` }]}>
                <ThemedText
                  type="default"
                  style={[styles.categoryText, { color: item.color }]}
                >
                  {item.category}
                </ThemedText>
              </View>
            </View>
          )}

          {item.issuedDate && (
            <View style={[styles.detailRow, { borderBottomColor: borderColor }]}>
              <ThemedText type="default" style={styles.detailLabel}>
                Issued Date
              </ThemedText>
              <ThemedText type="default" style={styles.detailValue}>
                {new Date(item.issuedDate).toLocaleDateString("en-US", {
                  year: "numeric",
                  month: "long",
                  day: "numeric",
                })}
              </ThemedText>
            </View>
          )}

          {item.expiryDate && (
            <View style={[styles.detailRow, { borderBottomColor: borderColor }]}>
              <ThemedText type="default" style={styles.detailLabel}>
                Expiry Date
              </ThemedText>
              <ThemedText type="default" style={styles.detailValue}>
                {new Date(item.expiryDate).toLocaleDateString("en-US", {
                  year: "numeric",
                  month: "long",
                  day: "numeric",
                })}
              </ThemedText>
            </View>
          )}

          <View style={styles.detailRow}>
            <ThemedText type="default" style={styles.detailLabel}>
              Status
            </ThemedText>
            <View style={styles.statusContainer}>
              {item.verified ? (
                <>
                  <MaterialIcons name="check-circle" size={20} color="#10B981" />
                  <ThemedText
                    type="defaultSemiBold"
                    style={[styles.statusText, { color: "#10B981" }]}
                  >
                    Verified
                  </ThemedText>
                </>
              ) : (
                <>
                  <MaterialIcons name="warning" size={20} color="#F59E0B" />
                  <ThemedText
                    type="defaultSemiBold"
                    style={[styles.statusText, { color: "#F59E0B" }]}
                  >
                    Unverified
                  </ThemedText>
                </>
              )}
            </View>
          </View>
        </ThemedView>

        {/* Actions Section */}
        <View style={styles.actionsSection}>
          <ThemedView
            style={[styles.actionButton, { backgroundColor: cardBackground }]}
          >
            <MaterialIcons name="share" size={24} color={item.color} />
            <ThemedText type="defaultSemiBold" style={{ color: item.color }}>
              Share
            </ThemedText>
          </ThemedView>
          <ThemedView
            style={[styles.actionButton, { backgroundColor: cardBackground }]}
          >
            <MaterialIcons name="download" size={24} color={item.color} />
            <ThemedText type="defaultSemiBold" style={{ color: item.color }}>
              Download
            </ThemedText>
          </ThemedView>
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
  headerCard: {
    padding: 24,
    borderRadius: 16,
    marginBottom: 24,
    flexDirection: "row",
    alignItems: "center",
    shadowColor: "#000",
    shadowOffset: { width: 0, height: 2 },
    shadowOpacity: 0.1,
    shadowRadius: 8,
    elevation: 3,
  },
  iconContainer: {
    width: 80,
    height: 80,
    borderRadius: 16,
    justifyContent: "center",
    alignItems: "center",
    marginRight: 16,
  },
  headerContent: {
    flex: 1,
  },
  titleRow: {
    flexDirection: "row",
    alignItems: "center",
    gap: 8,
    marginBottom: 4,
  },
  title: {
    fontSize: 24,
    flex: 1,
  },
  subtitle: {
    fontSize: 16,
    opacity: 0.7,
  },
  section: {
    marginBottom: 24,
  },
  sectionTitle: {
    fontSize: 18,
    marginBottom: 16,
  },
  detailRow: {
    flexDirection: "row",
    justifyContent: "space-between",
    alignItems: "flex-start",
    paddingVertical: 12,
  },
  detailLabel: {
    fontSize: 14,
    opacity: 0.7,
    flex: 1,
  },
  detailValue: {
    fontSize: 14,
    flex: 2,
    textAlign: "right",
  },
  categoryBadge: {
    paddingHorizontal: 12,
    paddingVertical: 4,
    borderRadius: 12,
  },
  categoryText: {
    fontSize: 14,
    fontWeight: "600",
  },
  statusContainer: {
    flexDirection: "row",
    alignItems: "center",
    gap: 6,
  },
  statusText: {
    fontSize: 14,
  },
  actionsSection: {
    flexDirection: "row",
    gap: 12,
    marginTop: 8,
  },
  actionButton: {
    flex: 1,
    flexDirection: "row",
    alignItems: "center",
    justifyContent: "center",
    gap: 8,
    padding: 16,
    borderRadius: 12,
    shadowColor: "#000",
    shadowOffset: { width: 0, height: 2 },
    shadowOpacity: 0.1,
    shadowRadius: 4,
    elevation: 2,
  },
});

