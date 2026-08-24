import { useEffect } from "react";
import { onSelection, onStream } from "./events";
import { applyMirrored, useChat } from "../stores/useChat";

/** What the other window did that this one has to know about: a run it started,
 *  so expanding mid-stream keeps the answer, and the model it chose for the
 *  next one. Both windows are views over the same core, and neither may sit
 *  showing state the other has moved on from. */
export function useMirroredWindow() {
  const adoptSelection = useChat((state) => state.adoptSelection);

  useEffect(() => onStream(applyMirrored), []);
  useEffect(
    () => onSelection(({ providerId, model }) => adoptSelection(providerId, model)),
    [adoptSelection],
  );
}
