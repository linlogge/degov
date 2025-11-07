import { createFileRoute } from "@tanstack/react-router";
import { Button } from "@dgv/ui/button";
import { SiteHeader } from "@/components/site-header";
import { Hello } from "@/components/hello";

export const Route = createFileRoute("/")({
  component: App,
});

function App() {
  const handleClick = () => {
    alert("Button from @dgv/ui package clicked!");
  };

  return (
    <>
      <SiteHeader />
      <div className="flex flex-1 flex-col">
        <div className="@container/main flex flex-1 flex-col gap-2">
          <div className="flex flex-col gap-4 py-4 md:gap-6 md:py-6">
            <Hello />
          </div>
        </div>
      </div>
    </>
  );
}
