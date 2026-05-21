const en = {
  common: {
    loading: "Loading...",
    send: "Send",
    sending: "Sending...",
    back: "Back",
    save: "Save",
    signOut: "Sign out",
    requestFailed: "request failed",
    configLoadFailed: "config load failed",
  },
  signIn: {
    title: "Support sign-in",
    tokenLabel: "Agent token",
    submit: "Sign in",
    failureFallback: "sign-in failed",
  },
  awaiting: {
    title: "Awaiting chats",
    startMockTest: "Start mock test",
    columns: {
      instance: "Instance",
      firstSeen: "First seen",
    },
    openAction: "Open",
    toast: (id: string) => `New chat ${id}`,
    errors: {
      mockTest: "mock test failed",
      poll: "poll failed",
    },
  },
  transcript: {
    agentTitle: "Agent transcript",
    mockTitle: "Mock test",
    empty: "No transcript yet",
    composer: {
      agent: "Reply as support",
      customer: "Write as customer",
    },
    role: {
      customer: "Customer",
      assistant: "Assistant",
    },
  },
  agentName: {
    promptTitle: "Agent name",
    fieldLabel: "Name shown in chat",
  },
  version: {
    updateAvailable: "A new chat UI version is available.",
    reload: "Reload",
  },
  brand: {
    agentLabel: "Agent",
    productLabel: "Chat",
  },
  language: {
    label: "Language",
    english: "English",
    japanese: "Japanese",
  },
} as const;

type WidenTranslations<T> = T extends string
  ? string
  : T extends (...args: infer Args) => string
    ? (...args: Args) => string
    : { readonly [Key in keyof T]: WidenTranslations<T[Key]> };

export type Translations = WidenTranslations<typeof en>;
export default en;
