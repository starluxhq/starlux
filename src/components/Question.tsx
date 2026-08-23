/** The user's own words, set against the answer rather than beside it: the
 *  question is contained and sits to the right, the answer opens to the left
 *  under the spectrum rail. Its rule is the rail's inert twin — same 2px, on
 *  the far edge, carrying no light of its own. */
export default function Question({ text }: { text: string }) {
  return (
    <div className="flex justify-end">
      <p className="max-w-[82%] rounded-l-[10px] rounded-r-[2px] border-r-2 border-faint bg-haze/70 px-3.5 py-2 text-[13.5px] leading-[1.6] break-words whitespace-pre-wrap text-ink/85 select-text">
        {text}
      </p>
    </div>
  );
}
