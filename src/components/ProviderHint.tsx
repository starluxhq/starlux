import type { Provider } from "../lib/types";

/** Stands in for the model trigger when nothing can be run. Installed-but-
 *  signed-out is the case worth naming separately: the fix is one command, not
 *  an install, and "no provider" sends the user looking for the wrong thing. */
export default function ProviderHint({ providers }: { providers: Provider[] }) {
  const signedOut = providers.find((provider) => provider.availability.state === "signedOut");

  return (
    <span
      title={
        signedOut
          ? `${signedOut.name} is installed but signed out. Run \`${signedOut.binary} login\` in a terminal.`
          : "No supported CLI was found on PATH."
      }
      className="shrink-0 font-mono text-[10px] whitespace-nowrap text-class-m uppercase"
    >
      {signedOut ? "signed out" : "no provider"}
    </span>
  );
}
