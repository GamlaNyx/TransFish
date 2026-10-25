const NAMES: Record<string, string> = {
  zh: "中文",
  en: "英语",
  ja: "日语",
  ko: "韩语",
  fr: "法语",
  de: "德语",
  es: "西班牙语",
  ru: "俄语",
  pt: "葡萄牙语",
  it: "意大利语",
  ar: "阿拉伯语",
  th: "泰语",
  vi: "越南语",
  id: "印尼语",
  ms: "马来语",
  tr: "土耳其语",
  nl: "荷兰语",
  pl: "波兰语",
  hi: "印地语",
  sv: "瑞典语",
  da: "丹麦语",
  fi: "芬兰语",
  no: "挪威语",
  cs: "捷克语",
  hu: "匈牙利语",
  ro: "罗马尼亚语",
  el: "希腊语",
  uk: "乌克兰语",
  bg: "保加利亚语",
  he: "希伯来语",
  fa: "波斯语",
  ur: "乌尔都语",
  bn: "孟加拉语",
  ta: "泰米尔语",
  mn: "蒙古语",
  kk: "哈萨克语",
  uz: "乌兹别克语",
  my: "缅甸语",
  km: "高棉语",
  lo: "老挝语",
  tl: "菲律宾语",
  sw: "斯瓦希里语",
  sr: "塞尔维亚语",
  hr: "克罗地亚语",
  sk: "斯洛伐克语",
  sl: "斯洛文尼亚语",
  et: "爱沙尼亚语",
  lv: "拉脱维亚语",
  lt: "立陶宛语",
  is: "冰岛语",
  af: "南非荷兰语",
  sq: "阿尔巴尼亚语",
  hy: "亚美尼亚语",
  ka: "格鲁吉亚语",
  az: "阿塞拜疆语",
  be: "白俄罗斯语",
  bs: "波斯尼亚语",
  mk: "马其顿语",
  mt: "马耳他语",
  ga: "爱尔兰语",
  cy: "威尔士语",
  eu: "巴斯克语",
  ca: "加泰罗尼亚语",
  gl: "加利西亚语",
};

export function langName(code?: string | null): string {
  if (!code) {
    return "自动";
  }
  const normalized = code.trim().replace(/_/g, "-");
  if (!normalized) {
    return "自动";
  }
  const lower = normalized.toLowerCase();
  if (lower === "zh-hant" || lower.includes("-tw") || lower.includes("-hk")) {
    return "繁体中文";
  }
  const base = lower.split("-")[0];
  return NAMES[base] ?? normalized.toUpperCase();
}

export function shortLang(code?: string | null): string {
  if (!code) {
    return "自动";
  }
  const base = code.trim().split(/[-_]/)[0];
  if (!base) {
    return "自动";
  }
  return base === "zh" ? "中" : base === "en" ? "EN" : base.toUpperCase();
}

export function formatLatency(ms: number): string {
  if (!Number.isFinite(ms) || ms <= 0) {
    return "—";
  }
  return ms >= 1000 ? `${(ms / 1000).toFixed(1)}s` : `${Math.round(ms)}ms`;
}

export const TARGET_OPTIONS: { value: string; label: string }[] = [
  { value: "auto", label: "自动（中文 ↔ 英文，其余译为中文）" },
  { value: "zh", label: "中文" },
  { value: "en", label: "英语" },
  { value: "ja", label: "日语" },
  { value: "ko", label: "韩语" },
  { value: "fr", label: "法语" },
  { value: "de", label: "德语" },
  { value: "es", label: "西班牙语" },
  { value: "ru", label: "俄语" },
];
