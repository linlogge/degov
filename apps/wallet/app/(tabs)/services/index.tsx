import { Stack } from "expo-router";
import { Button, ScrollView, Text, View } from "react-native";

export default function ServicesPage() {
    return (
        <ScrollView>
            <Stack.Screen
                options={{
                    title: "Services",
                    headerSearchBarOptions: {
                        placeholder: "Search...",
                        placement: "automatic",
                    },
                }}
            />
            <View>
                <Text>Setting1</Text>
                <Text>Setting2</Text>
                <Text>Setting3</Text>
                <Text>Setting4</Text>
                <Text>Setting5</Text>
            </View>
        </ScrollView>
    );
}
