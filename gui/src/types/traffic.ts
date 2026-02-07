export interface TrafficSummary {
  totalBytesPerSec: number;
  totalPacketsPerSec: number;
  topFlows: FlowEntry[];
  counterDeltas: CounterDelta[];
}

export interface FlowEntry {
  src: string;
  dst: string;
  protocol: string;
  bytesPerSec: number;
  packetsPerSec: number;
}

export interface CounterDelta {
  family: string;
  table: string;
  chain: string;
  handle: number;
  packetsPerSec: number;
  bytesPerSec: number;
}

export interface LiveFlowEntry extends FlowEntry {
  key: string;
  lastSeenAt: number;
  expiresAt: number;
}

export interface LiveCounterDelta extends CounterDelta {
  key: string;
  lastSeenAt: number;
  expiresAt: number;
}
