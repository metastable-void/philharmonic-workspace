import type { Translations } from "./en";

const ja: Translations = {
  common: {
    loading: "読み込み中...",
    send: "送信",
    sending: "送信中...",
    back: "戻る",
    save: "保存",
    signOut: "サインアウト",
    requestFailed: "リクエストに失敗しました",
    configLoadFailed: "設定の読み込みに失敗しました",
  },
  signIn: {
    title: "サポート担当者サインイン",
    tokenLabel: "担当者トークン",
    submit: "サインイン",
    failureFallback: "サインインに失敗しました",
  },
  awaiting: {
    title: "対応待ちチャット",
    startMockTest: "模擬テストを開始",
    columns: {
      instance: "インスタンス",
      firstSeen: "初回検知",
    },
    openAction: "開く",
    toast: (id: string) => `新しいチャット ${id}`,
    errors: {
      mockTest: "模擬テストに失敗しました",
      poll: "ポーリングに失敗しました",
    },
  },
  transcript: {
    agentTitle: "担当者トランスクリプト",
    mockTitle: "模擬テスト",
    empty: "トランスクリプトはまだありません",
    composer: {
      agent: "サポート担当として返信",
      customer: "顧客として入力",
    },
    role: {
      customer: "顧客",
      assistant: "アシスタント",
    },
  },
  agentName: {
    promptTitle: "担当者名",
    fieldLabel: "チャットに表示する名前",
  },
  version: {
    updateAvailable: "新しいチャットUIが利用できます。",
    reload: "再読み込み",
  },
  brand: {
    agentLabel: "担当者",
    productLabel: "チャット",
  },
  language: {
    label: "表示言語",
    english: "English",
    japanese: "日本語",
  },
};

export default ja;
