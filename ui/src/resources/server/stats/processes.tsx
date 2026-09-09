import { useRead } from "@/lib/hooks";
import { filterBySplit } from "mogh_ui";
import { ICONS } from "@/lib/icons";
import { DataTable, SortableHeader } from "mogh_ui";
import { SearchInput } from "mogh_ui";
import { Section } from "mogh_ui";
import { ShowHideButton } from "mogh_ui";
import { TableSkeleton } from "mogh_ui";
import { Group, Select } from "@mantine/core";
import { useState, useMemo } from "react";
import { Types } from "komodo_client";
import { useLocalStorage } from "@mantine/hooks";

export default function ServerProcesses({ id }: { id: string }) {
  const [show, setShow] = useState(false);
  const [search, setSearch] = useState("");
  const [interval, setInterval] = useLocalStorage<Types.Timelength>({
    key: "server-processes-interval-v1",
    defaultValue: Types.Timelength.FiveSeconds,
  });

  const refetchInterval = useMemo(() => {
    switch (interval) {
      case Types.Timelength.OneSecond:
        return 1_000;
      case Types.Timelength.TwoSeconds:
        return 2_000;
      case Types.Timelength.ThreeSeconds:
        return 3_000;
      case Types.Timelength.FiveSeconds:
        return 5_000;
      case Types.Timelength.FifteenSeconds:
        return 15_000;
      case Types.Timelength.ThirtySeconds:
        return 30_000;
      case Types.Timelength.OneMinute:
        return 60_000;
      case Types.Timelength.FiveMinutes:
        return 300_000;
      case Types.Timelength.FifteenMinutes:
        return 900_000;
      case Types.Timelength.ThirtyMinutes:
        return 1_800_000;
      case Types.Timelength.OneHour:
        return 3_600_000;
      case Types.Timelength.SixHours:
        return 21_600_000;
      case Types.Timelength.OneDay:
        return 86_400_000;
      default:
        return 5_000;
    }
  }, [interval]);

  const { data: processes, isPending } = useRead(
    "ListSystemProcesses",
    {
      server: id,
    },
    { enabled: show, refetchInterval },
  );

  const filtered = filterBySplit(processes, search, (item) => item.name);

  return (
    <Section
      withBorder
      title="Processes"
      icon={<ICONS.Process size="1.3rem" />}
      titleRight={
        <Group ml={{ sm: "xl" }} onClick={(e) => e.stopPropagation()}>
          <Select
            value={interval}
            onChange={(val) => val && setInterval(val as Types.Timelength)}
            data={[
              Types.Timelength.OneSecond,
              Types.Timelength.TwoSeconds,
              Types.Timelength.ThreeSeconds,
              Types.Timelength.FiveSeconds,
              Types.Timelength.FifteenSeconds,
              Types.Timelength.ThirtySeconds,
              Types.Timelength.OneMinute,
              Types.Timelength.FiveMinutes,
              Types.Timelength.FifteenMinutes,
              Types.Timelength.ThirtyMinutes,
              Types.Timelength.OneHour,
              Types.Timelength.SixHours,
              Types.Timelength.OneDay,
            ]}
            w={120}
          />
          <SearchInput
            value={search}
            onSearch={setSearch}
            w={{ base: 200, lg: 300 }}
          />
          <ShowHideButton show={show} setShow={setShow} />
        </Group>
      }
      onHeaderClick={() => setShow((s) => !s)}
    >
      {show && isPending && !processes && <TableSkeleton />}
      {show && !isPending && (
        <DataTable
          sortDescFirst
          mah="min(400px, calc(100vh - 320px))"
          tableKey="server-processes"
          data={filtered ?? []}
          columns={[
            {
              header: "Name",
              accessorKey: "name",
            },
            {
              header: "Exe",
              accessorKey: "exe",
              cell: ({ row }) => (
                <div className="overflow-hidden overflow-ellipsis">
                  {row.original.exe}
                </div>
              ),
            },
            {
              accessorKey: "cpu_perc",
              header: ({ column }) => (
                <SortableHeader column={column} title="CPU" sortDescFirst />
              ),
              cell: ({ row }) => <>{row.original.cpu_perc.toFixed(2)}%</>,
            },
            {
              accessorKey: "mem_mb",
              header: ({ column }) => (
                <SortableHeader column={column} title="Memory" sortDescFirst />
              ),
              cell: ({ row }) => (
                <>
                  {row.original.mem_mb > 1000
                    ? `${(row.original.mem_mb / 1024).toFixed(2)} GB`
                    : `${row.original.mem_mb.toFixed(2)} MB`}
                </>
              ),
            },
          ]}
        />
      )}
    </Section>
  );
}
