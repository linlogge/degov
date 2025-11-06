import MaterialIcons from "@expo/vector-icons/MaterialIcons";

export interface WalletItem {
  id: string;
  title: string;
  subtitle: string;
  icon: keyof typeof MaterialIcons.glyphMap;
  color: string;
  verified?: boolean;
  description?: string;
  issuer?: string;
  issuedDate?: string | null;
  expiryDate?: string | null;
  category?: string;
}

export const walletItems: WalletItem[] = [
  {
    id: "1",
    title: "Driver's License",
    subtitle: "Class B • Expires 2028",
    icon: "credit-card",
    color: "#3B82F6",
    verified: true,
    description: "Valid driver's license for operating passenger vehicles (Class B).",
    issuer: "Department of Motor Vehicles",
    issuedDate: "2020-03-15",
    expiryDate: "2028-03-15",
    category: "Identity",
  },
  {
    id: "2",
    title: "National ID Card",
    subtitle: "Identity Document",
    icon: "badge",
    color: "#10B981",
    verified: true,
    description: "Official national identification document issued by the government.",
    issuer: "National Identity Authority",
    issuedDate: "2019-06-20",
    expiryDate: "2029-06-20",
    category: "Identity",
  },
  {
    id: "3",
    title: "Health Certificate",
    subtitle: "COVID-19 Vaccination",
    icon: "medical-services",
    color: "#EF4444",
    verified: true,
    description: "COVID-19 vaccination certificate showing full vaccination status.",
    issuer: "Ministry of Health",
    issuedDate: "2021-07-10",
    expiryDate: null,
    category: "Health",
  },
  {
    id: "4",
    title: "University Diploma",
    subtitle: "Computer Science • 2023",
    icon: "school",
    color: "#8B5CF6",
    verified: true,
    description: "Bachelor of Science in Computer Science, awarded with honors.",
    issuer: "State University",
    issuedDate: "2023-05-15",
    expiryDate: null,
    category: "Education",
  },
  {
    id: "5",
    title: "Gym Membership",
    subtitle: "Active • Premium Plan",
    icon: "fitness-center",
    color: "#F59E0B",
    verified: false,
    description: "Premium membership with access to all facilities and classes.",
    issuer: "Fitness Plus",
    issuedDate: "2024-01-01",
    expiryDate: "2024-12-31",
    category: "Membership",
  },
  {
    id: "6",
    title: "Professional License",
    subtitle: "Software Engineer • Valid",
    icon: "workspace-premium",
    color: "#06B6D4",
    verified: true,
    description: "Licensed professional software engineer certification.",
    issuer: "Professional Engineering Board",
    issuedDate: "2022-09-01",
    expiryDate: "2025-09-01",
    category: "Professional",
  },
];

// Helper function to get wallet item by ID
export function getWalletItemById(id: string): WalletItem | undefined {
  return walletItems.find((item) => item.id === id);
}

// Helper function to create a lookup map for O(1) access
export function getWalletItemsMap(): Record<string, WalletItem> {
  return walletItems.reduce((acc, item) => {
    acc[item.id] = item;
    return acc;
  }, {} as Record<string, WalletItem>);
}




