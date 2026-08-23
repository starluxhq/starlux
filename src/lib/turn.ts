import { createContext, useContext } from "react";
import type { Status, Turn } from "../stores/useChat";

/** Streamdown matches a renderer on the fence language and hands it the body,
 *  so a widget that needs to know which turn it belongs to has to be told. */
export const TurnContext = createContext<string>("");

export const useTurnId = () => useContext(TurnContext);

/** Which rail a turn gets. Only the run still going shows the travelling
 *  spectrum — every earlier answer keeps the hairline it finished with. */
export function railState(turn: Turn, runId: string | null, status: Status): Status {
  if (turn.error) return "error";
  return turn.id === runId ? status : "idle";
}
