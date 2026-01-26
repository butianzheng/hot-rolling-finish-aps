import { listen, UnlistenFn } from '@tauri-apps/api/event';
import { useEffect, useRef } from 'react';

type EventHandler = (payload: any) => void;

export class EventBus {
  private static listeners: Map<string, UnlistenFn[]> = new Map();

  static async subscribe(
    eventName: string,
    handler: EventHandler
  ): Promise<UnlistenFn> {
    const unlisten = await listen(eventName, (event) => {
      handler(event.payload);
    });

    if (!this.listeners.has(eventName)) {
      this.listeners.set(eventName, []);
    }
    this.listeners.get(eventName)!.push(unlisten);

    return unlisten;
  }

  static async unsubscribe(eventName: string) {
    const listeners = this.listeners.get(eventName);
    if (listeners) {
      listeners.forEach(unlisten => unlisten());
      this.listeners.delete(eventName);
    }
  }

  static async cleanup() {
    for (const listeners of this.listeners.values()) {
      listeners.forEach(unlisten => unlisten());
    }
    this.listeners.clear();
  }
}

export function useEvent(eventName: string, handler: EventHandler) {
  // Keep a stable subscription while still calling the latest handler.
  const handlerRef = useRef<EventHandler>(handler);
  useEffect(() => {
    handlerRef.current = handler;
  }, [handler]);

  useEffect(() => {
    let didCleanup = false;
    let unlisten: UnlistenFn | undefined;

    EventBus.subscribe(eventName, (payload: any) => handlerRef.current(payload)).then((fn) => {
      if (didCleanup) {
        // If the component unmounted before the async subscribe finished, immediately cleanup.
        fn();
        return;
      }
      unlisten = fn;
    });

    return () => {
      didCleanup = true;
      if (unlisten) unlisten();
    };
  }, [eventName]);
}
