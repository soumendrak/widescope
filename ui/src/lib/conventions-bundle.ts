import otelRaw from '../../../conventions/opentelemetry.json?raw';
import openinferenceRaw from '../../../conventions/openinference.json?raw';
import langchainRaw from '../../../conventions/langchain.json?raw';
import pricingRaw from '../../../conventions/pricing.json?raw';

export const BUNDLED_CONVENTIONS: string[] = [otelRaw, openinferenceRaw, langchainRaw];
export const BUNDLED_PRICING: string = pricingRaw;
