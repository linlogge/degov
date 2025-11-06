import { Stack, useNavigation, useRouter } from "expo-router";
import { useEffect, useRef } from "react";
import { Button, ScrollView, Text, View } from "react-native";

export default function ServicesPage() {
    const router = useRouter();

    return (
        <>
            <ScrollView>
                <Stack.Screen
                    options={{
                        title: "Search",
                        headerSearchBarOptions: {
                            placeholder: "Search...",
                            placement: "automatic"
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
        </>
    );
}
