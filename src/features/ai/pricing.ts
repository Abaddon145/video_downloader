
import type {UsageModel} from '../../types/ai';
// USD per million tokens. Official rate card checked 2026-10-17.
// Actual peak/off-peak periods, discounts and account currency remain billing facts at DeepSeek.
export const pricing={source:'https://api-docs.deepseek.com/quick_start/pricing/',checkedAt:'2026-10-17',rates:{
 'deepseek-flash':{input:[0.15,0.30],cached:[0.003,0.006],output:[0.60,1.20]},
 'deepseek-v4-pro':{input:[0.66,1.32],cached:[0.022,0.044],output:[1.98,3.96]},
}};
export function estimateCost(usage:UsageModel):{min:number;max:number}|null{
 const rates=pricing.rates[usage.model as keyof typeof pricing.rates];if(!rates||usage.unknownRequests||usage.cachedTokens>usage.inputTokens)return null;
 const price=(i:number)=>((usage.inputTokens-usage.cachedTokens)*rates.input[i]+usage.cachedTokens*rates.cached[i]+usage.outputTokens*rates.output[i])/1000000;
 return {min:price(0),max:price(1)};
}
