// Path: app/src/crosstalk/conversation.ts
// Description: Bounded recent pair context for recalling an expired message beside its reply

import type { Message } from "./identity.js";

export interface SpeechMessage extends Message { echo?: Message }
interface Recent { message: Message; receivedAt: number }
export interface Conversation { cards: SpeechMessage[]; recent: Recent[] }

export function receive(state: Conversation, message: Message, held: ReadonlySet<string>, now: number): Conversation {
  if (state.recent.some(item => item.message.id === message.id) || state.cards.some(item => item.id === message.id)) return state;
  const replace = state.cards.length < 3 ? -1 : state.cards.findIndex(item => !held.has(item.id));
  if (state.cards.length === 3 && replace < 0) return state;
  const samePair = (item: Recent): boolean =>
    (item.message.from === message.to && item.message.to === message.from)
    || (item.message.from === message.from && item.message.to === message.to);
  const recent = state.recent.filter(item => now - item.receivedAt < 300_000);
  const previous = recent.find(samePair)?.message;
  const echo = previous && previous.from !== message.from && previous.from === message.to
    && !state.cards.some(item => item.id === previous.id) ? previous : undefined;
  return {
    cards: [...state.cards.filter((_, index) => index !== replace), echo ? { ...message, echo } : message],
    recent: [{ message, receivedAt: now }, ...recent.filter(item => !samePair(item))].slice(0, 16),
  };
}
