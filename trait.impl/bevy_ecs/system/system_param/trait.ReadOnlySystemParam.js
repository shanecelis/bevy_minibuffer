(function() {
    const implementors = Object.fromEntries([["bevy_minibuffer",[["impl&lt;'w, 's&gt; ReadOnlySystemParam for <a class=\"struct\" href=\"bevy_minibuffer/struct.Minibuffer.html\" title=\"struct bevy_minibuffer::Minibuffer\">Minibuffer</a>&lt;'w, 's&gt;<div class=\"where\">where\n    Query&lt;'w, 's, Entity, With&lt;PromptContainer&gt;&gt;: ReadOnlySystemParam,\n    Commands&lt;'w, 's&gt;: ReadOnlySystemParam,\n    Res&lt;'w, State&lt;<a class=\"enum\" href=\"bevy_minibuffer/prompt/enum.PromptState.html\" title=\"enum bevy_minibuffer::prompt::PromptState\">PromptState</a>&gt;&gt;: ReadOnlySystemParam,\n    ResMut&lt;'w, NextState&lt;<a class=\"enum\" href=\"bevy_minibuffer/prompt/enum.PromptState.html\" title=\"enum bevy_minibuffer::prompt::PromptState\">PromptState</a>&gt;&gt;: ReadOnlySystemParam,\n    ResMut&lt;'w, <a class=\"enum\" href=\"bevy_minibuffer/acts/tape/enum.TapeRecorder.html\" title=\"enum bevy_minibuffer::acts::tape::TapeRecorder\">TapeRecorder</a>&gt;: ReadOnlySystemParam,</div>",0]]]]);
    if (window.register_implementors) {
        window.register_implementors(implementors);
    } else {
        window.pending_implementors = implementors;
    }
})()
//{"start":59,"fragment_lengths":[970]}