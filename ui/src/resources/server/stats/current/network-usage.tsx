import { InfoCard } from "mogh_ui";
import { Group, Stack, Text } from "@mantine/core";
import { Types } from "komodo_client";
import { fmtRateBytes } from "mogh_ui";

export function formatNetworkRate(bytesPerSec: number): string {
  if (bytesPerSec >= 1024 * 1024 * 1024) {
    return `${(bytesPerSec / (1024 * 1024 * 1024)).toFixed(1)} GiB/s`;
  }
  if (bytesPerSec >= 1024 * 1024) {
    return `${(bytesPerSec / (1024 * 1024)).toFixed(1)} MiB/s`;
  }
  if (bytesPerSec >= 1024) {
    return `${(bytesPerSec / 1024).toFixed(1)} KiB/s`;
  }
  return `${Math.round(bytesPerSec)} bytes/s`;
}

export default function ServerNetworkUsage({
  stats,
}: {
  stats: Types.SystemStats | undefined;
}) {
  return (
    <InfoCard title="Network Usage" w={{ base: "100%", lg: 300 }} gap="xs">
      <Stack gap="0">
        <Group justify="space-between">
          <Text>Ingress</Text>
          <Text>{formatNetworkRate(stats?.network_ingress_bytes ?? 0)}</Text>
        </Group>
        <Group justify="space-between">
          <Text>Egress</Text>
          <Text>{formatNetworkRate(stats?.network_egress_bytes ?? 0)}</Text>
        </Group>
      </Stack>
    </InfoCard>
  );
}
