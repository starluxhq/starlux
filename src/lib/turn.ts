import { createContext, useContext } from "react";

/** Streamdown matches a renderer on the fence language and hands it the body,
 *  so a widget that needs to know which turn it belongs to has to be told. */
export const TurnContext = createContext<string>("");

export const useTurnId = () => useContext(TurnContext);
