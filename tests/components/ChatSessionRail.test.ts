import { describe, expect, it } from "vitest";
import { mount } from "@vue/test-utils";
import ChatSessionRail from "@/components/chat/ChatSessionRail.vue";
import type { ChatSession } from "@/lib/agentChat/chatSessions";

function session(over: Partial<ChatSession> = {}): ChatSession {
  return {
    id: "s1",
    title: "First chat",
    titleIsDefault: false,
    createdAt: 1,
    updatedAt: Date.now(),
    messages: [],
    ...over,
  };
}

describe("ChatSessionRail", () => {
  it("lists sessions and emits new/select/remove", async () => {
    const wrapper = mount(ChatSessionRail, {
      props: {
        sessions: [session(), session({ id: "s2", title: "Second chat" })],
        activeSessionId: "s1",
        renamingId: null,
        renameValue: "",
      },
    });

    expect(wrapper.find('[aria-label="Chat sessions"]').exists()).toBe(true);
    expect(wrapper.text()).toContain("First chat");
    expect(wrapper.text()).toContain("Second chat");

    await wrapper.get(".chat__rail-new").trigger("click");
    expect(wrapper.emitted("new")).toHaveLength(1);

    await wrapper.findAll(".chat__rail-item")[1].trigger("click");
    expect(wrapper.emitted("select")?.[0]).toEqual(["s2"]);

    await wrapper.findAll('[aria-label="Delete chat"]')[0].trigger("click");
    expect(wrapper.emitted("remove")?.[0]).toEqual(["s1"]);
  });
});
