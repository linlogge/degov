import { DataTable } from '@/components/data-table';
import { SiteHeader } from '@/components/site-header';
import { createFileRoute } from '@tanstack/react-router'

import data from './data.json';
import { useQuery } from '@degov/sdk/query';
import { EngineService } from '@degov/sdk/pb/engine_pb';

export const Route = createFileRoute('/workflows/')({
  component: RouteComponent,
})

function RouteComponent() {
  const { data, isLoading, error } = useQuery(EngineService.method.listWorkers);
  
  return (
    <>
      <SiteHeader />
      <div className="flex flex-1 flex-col">
        <div className="@container/main flex flex-1 flex-col gap-2">
          <div className="flex flex-col gap-4 py-4 md:gap-6 md:py-6">
            {JSON.stringify(data)}
          </div>
        </div>
      </div>
    </>
  );
}
