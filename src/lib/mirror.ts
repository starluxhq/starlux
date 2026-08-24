import { useEffect } from "react";
import { onSelection, onStream, onTools } from "./events";
import { applyMirrored, useChat } from "../stores/useChat";
import { useSettings } from "../stores/useSettings";

/** What the other window did that this one has to know about: a run it started,
 *  so expanding mid-stream keeps the answer, the model it chose for the next
 *  one, and what it granted the tools to reach. Both windows are views over the
 *  same core, and neither may sit showing state the other has moved on from. */
export function useMirroredWindow() {
  const adoptSelection = useChat((state) => state.adoptSelection);
  const adoptTools = useSettings((state) => state.adoptTools);

  useEffect(() => onStream(applyMirrored), []);
  useEffect(
    () => onSelection(({ providerId, model }) => adoptSelection(providerId, model)),
    [adoptSelection],
  );
  useEffect(() => onTools(adoptTools), [adoptTools]);
}
