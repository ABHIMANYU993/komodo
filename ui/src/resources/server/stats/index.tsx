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
    key: "server-current-stats-interval-v2",
    defaultValue: Types.Timelength.OneSecond,
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
        return 1_000;
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
