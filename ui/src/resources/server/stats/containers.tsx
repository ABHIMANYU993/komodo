import { useRead } from "@/lib/hooks";
import { DataTable, SortableHeader } from "mogh_ui";
import { Section } from "mogh_ui";
import { ShowHideButton } from "mogh_ui";
import { Group, Select, Text } from "@mantine/core";
import { useLocalStorage } from "@mantine/hooks";
import { useState, useMemo } from "react";
import { filterBySplit } from "mogh_ui";
import DockerResourceLink from "@/components/docker/link";
import { useIsServerAvailable } from "../hooks";
import { SearchInput } from "mogh_ui";
import { ICONS } from "@/lib/icons";
import { Types } from "komodo_client";

export default function ServerContainerStats({ id }: { id: string }) {
  const [search, setSearch] = useState("");
  const [show, setShow] = useLocalStorage({
    key: "server-stats-containers-show-v2",
    defaultValue: true,
  });
  const [interval, setInterval] = useLocalStorage<Types.Timelength>({
    key: "server-containers-interval-v1",
    defaultValue: Types.Timelength.FifteenSeconds,
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
      default:
        return 15_000;
    }
  }, [interval]);

  const isServerAvailable = useIsServerAvailable(id);
  const containers = useRead(
    "ListContainers",
    {
      server: id,
    },
    {
      enabled: isServerAvailable && show,
      refetchInterval,
    },
  ).data?.filter((c) => c.stats);
  const filtered = filterBySplit(
    containers,
    search,
    (container) => container.name,
  );
  return (
    <Section
      withBorder
      title="Containers"
      icon={<ICONS.Container size="1.3rem" />}
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
            ]}
            w={120}
          />
          <SearchInput
            value={search}
            onSearch={setSearch}
            w={{ base: 180, lg: 240 }}
          />
          <ShowHideButton show={show} setShow={setShow} />
        </Group>
      }
      onHeaderClick={() => setShow((s) => !s)}
    >
      {show && (
        <DataTable
          sortDescFirst
          mah="min(400px, calc(100vh - 320px))"
          tableKey="container-stats"
          data={filtered}
          columns={[
            {
              accessorKey: "name",
              header: ({ column }) => (
                <SortableHeader column={column} title="Name" />
              ),
              cell: ({ row }) => (
                <DockerResourceLink
                  type="Container"
                  serverId={id}
                  name={row.original.name}
                />
              ),
            },
            {
              accessorKey: "stats.cpu_perc",
              header: ({ column }) => (
                <SortableHeader column={column} title="CPU" />
              ),
            },
            {
              accessorKey: "stats.mem_perc",
              header: ({ column }) => (
                <SortableHeader column={column} title="Memory" />
              ),
              cell: ({ row }) => (
                <Group wrap="nowrap" gap="xs">
                  <Text>{row.original.stats?.mem_perc}</Text>
                  <Text c="muted" size="sm">
                    ({row.original.stats?.mem_usage})
                  </Text>
                </Group>
              ),
            },
            {
              accessorKey: "stats.net_io",
              header: ({ column }) => (
                <SortableHeader column={column} title="Net I/O" />
              ),
            },
            {
              accessorKey: "stats.block_io",
              header: ({ column }) => (
                <SortableHeader column={column} title="Block I/O" />
              ),
            },
            {
              accessorKey: "stats.pids",
              header: ({ column }) => (
                <SortableHeader column={column} title="PIDs" />
              ),
            },
          ]}
        />
      )}
    </Section>
  );
}
