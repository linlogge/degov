import { DataTable } from '@/components/data-table';
import { SiteHeader } from '@/components/site-header';
import { createFileRoute } from '@tanstack/react-router'

import data from './data.json';

export const Route = createFileRoute('/workflows/')({
  component: RouteComponent,
})

function RouteComponent() {
  return (
    <>
      <SiteHeader />
      <div className="flex flex-1 flex-col">
        <div className="@container/main flex flex-1 flex-col gap-2">
          <div className="flex flex-col gap-4 py-4 md:gap-6 md:py-6">
            <DataTable data={data} />
          </div>
        </div>
      </div>
    </>
  );
}
