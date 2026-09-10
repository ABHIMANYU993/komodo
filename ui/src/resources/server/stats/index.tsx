import { lazy, ReactNode, Suspense, useMemo } from "react";
import { usePermissions, useRead } from "@/lib/hooks";
import { Types } from "komodo_client";
import { Section } from "mogh_ui";
import { useLocalStorage } from "@mantine/hooks";
import ServerProcesses from "./processes";
import ServerContainerStats from "./containers";
import ServerDisks from "./disks";
import ServerCurrentStats from "./current";
import ServerSystemInfo from "./system-info";
import { useIsServerAvailable } from "../hooks";

// Loaded lazily to keep recharts out of the entry chunk.
const ServerHistoricalStats = lazy(() => import("./historical"));

export default function ServerStats({
  id,
  titleOther,
}: {
  id: string;
  titleOther?: ReactNode;
}) {
  const { specific } = usePermissions({ type: "Server", id });
  const isServerAvailable = useIsServerAvailable(id);

  const [interval, setInterval] = useLocalStorage<Types.Timelength>({
    key: "server-current-stats-interval-v1",
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
      default:
        return 5_000;
    }
  }, [interval]);

  const stats = useRead(
    "GetSystemStats",
    { server: id },
    {
      enabled: isServerAvailable,
      refetchInterval,
    },
  ).data;

  return (
    <Section titleOther={titleOther} gap="2.5rem">
      <ServerSystemInfo id={id} stats={stats} />

      <ServerCurrentStats
        id={id}
        stats={stats}
        interval={interval}
        setInterval={setInterval}
      />

      <Suspense fallback={null}>
        <ServerHistoricalStats id={id} />
      </Suspense>

      <ServerContainerStats id={id} />

      <ServerDisks stats={stats} />

      {specific.includes(Types.SpecificPermission.Processes) && (
        <ServerProcesses id={id} />
      )}
    </Section>
  );
}
