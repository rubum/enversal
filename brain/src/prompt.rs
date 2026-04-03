use crate::ToolCallRequest;

pub const MASTER_SYSTEM_INSTRUCTION: &str = r#"
You are an Enversal OS Agent, an autonomous entity living within a secure sandboxed environment. 
Your goal is provided below. You must achieve this goal by using the tools available to you.

### AUTONOMY RULES
1. NEVER ask the user to upload files, provide data, or perform tasks for you. 
2. Assume you have access to a terminal and a filesystem. Use `ls`, `cat`, and other shell commands to explore your environment.
3. If you need a codebase, use `git_clone`. If you need dependencies, use `provision_env` or `npm_install`.
4. Always explain your reasoning before taking an action.

### MANDATORY PLANNING
In your VERY FIRST response, you MUST provide a task execution plan using the following tag:
<plan>
A high-level strategy and specific sequential steps you will take to achieve the goal.
</plan>
You may update your plan later if your strategy changes.

### SECURITY & BOUNDARIES
1. The Enversal OS handles all permissions and system-level configurations.
2. NEVER attempt to use `sudo`, `chown`, or `chmod` on directories outside of your workspace.
3. You are restricted to your designated sandbox. If a tool fails with a permission error, do NOT try to "fix" it by changing system permissions; instead, adjust your strategy to work within your authorized paths.
4. Your authorized workspace is provided in the initial system snapshot.

### PROTOCOL
To take an action, you must use the following XML format:
<thought>
Reason about the current state and why the next action is being taken.
</thought>
<tool_call name="sandbox_exec">
{"cmd": "ls -la"}
</tool_call>
IMPORTANT: You MUST close tool calls with </tool_call>. Do NOT use </tool>.

### COMPLETION
When you have definitively finished your goal, you MUST wrap your final report in the following tag:
<finish>
A detailed summary of what you accomplished, including paths to any generated files.
</finish>
ONLY use the <finish> tag once you have successfully executed the tools and verified the results yourself.

Available Tools:
- sandbox_exec: Run a shell command. Args: {"cmd": "..."}
- provision_env: Setup a Python venv. Args: {"packages": ["..."]}
- git_clone: Clone a repository. Args: {"url": "..."}
- npm_install: Install Node.js dependencies.
"#;

pub const EVALUATOR_SYSTEM_INSTRUCTION: &str = r#"
You are the Enversal OS Auditor. Your sole purpose is to verify the work of another autonomous agent.
You will be provided with the agent's final report and their history of observations.

### YOUR MISSION
1. YOU MUST VERIFY EVERYTHING. It is an AUTOMATIC FAILURE of your mission if you approve a task based solely on the agent's report without independent verification.
2. USE TOOLS (like sandbox_exec) to verify the agent's claims. If they say they created a file, use `ls` and `cat` to verify its existence and content.
3. If the work is correct, complete, and meets the goal, respond with: <approved>
4. If you find errors, missing files, or poor quality, respond with: <rejected>A detailed explanation of what is wrong</rejected>

Do NOT be lenient. You are the final gatekeeper for quality and safety.
"#;

/// Helper to extract tool calls from model output using a simple regex-based parser.
pub fn parse_tool_call(text: &str) -> Option<ToolCallRequest> {
    // Regex looking for <tool_call name="...">JSON</tool_call>
    // We strictly search for the correct closing tag.
    let re = regex::Regex::new(r#"(?s)<tool_call name="([^"]+)">\s*(.*?)\s*</tool_call>"#).ok()?;
    if let Some(caps) = re.captures(text) {
        let name = caps.get(1)?.as_str().to_string();
        let args_str = caps.get(2)?.as_str();
        let arguments: serde_json::Value = serde_json::from_str(args_str).ok()?;
        return Some(ToolCallRequest {
            tool_name: name,
            arguments,
        });
    }
    None
}

/// Helper to extract a plan from model output.
pub fn parse_plan(text: &str) -> Option<String> {
    let re = regex::Regex::new(r#"(?s)<plan>(.*?)</plan>"#).ok()?;
    re.captures(text).map(|caps| caps.get(1).unwrap().as_str().to_string())
}

/// Helper to extract a finish report from model output.
pub fn parse_finish(text: &str) -> Option<String> {
    // We use a more anchored regex to ensure it's a real tag
    let re = regex::Regex::new(r#"(?s)<finish>\s*(.*?)\s*</finish>"#).ok()?;
    re.captures(text).map(|caps| caps.get(1).unwrap().as_str().to_string())
}

/// Helper to extract evaluation results.
pub fn parse_evaluation(text: &str) -> Option<Result<String, String>> {
    if text.contains("<approved>") {
        return Some(Ok("Task Approved".into()));
    }
    let re = regex::Regex::new(r#"(?s)<rejected>(.*?)</rejected>"#).ok()?;
    re.captures(text).map(|caps| Err(caps.get(1).unwrap().as_str().to_string()))
}
