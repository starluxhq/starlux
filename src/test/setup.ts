import { cleanup } from "@testing-library/react";
import { afterEach } from "vitest";

// Without `globals`, React Testing Library never registers its own.
afterEach(cleanup);
