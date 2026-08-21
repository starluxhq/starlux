import { create } from "zustand";

export interface Artifact {
  html: string;
  title: string;
}

interface ArtifactState {
  expanded: Artifact | null;
  expand: (artifact: Artifact) => void;
  collapse: () => void;
}

/** Which artifact, if any, has been pulled out of the thread into its own pane.
 *  Window-local: the Quick Bar has nowhere to put one and never reads this. */
export const useArtifact = create<ArtifactState>((set) => ({
  expanded: null,
  expand: (expanded) => set({ expanded }),
  collapse: () => set({ expanded: null }),
}));
