import { createFileRoute } from "@tanstack/react-router";
import { Button } from "@degov/ui/button";
import { SiteHeader } from "@/components/site-header";

export const Route = createFileRoute("/")({
  component: App,
});

function App() {
  const handleClick = () => {
    alert("Button from @degov/ui package clicked!");
  };

  return (
    <>
      <SiteHeader />
      <div className="flex flex-1 flex-col">
        <div className="@container/main flex flex-1 flex-col gap-2">
          <div className="flex flex-col gap-4 py-4 md:gap-6 md:py-6">
          </div>
        </div>
      </div>
    </>
  );
}
