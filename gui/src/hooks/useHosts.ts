import { useEffect } from "react";
import { daemonCall } from "./useDaemon";
import { useHostsStore } from "../stores/hosts";
import type { RemoteHost } from "../types/hosts";

export function useHosts() {
  const setHosts = useHostsStore((s) => s.setHosts);

  useEffect(() => {
    void (async () => {
      const hosts = await daemonCall<RemoteHost[]>("list_hosts");
      setHosts(hosts);
    })();
  }, [setHosts]);
}
