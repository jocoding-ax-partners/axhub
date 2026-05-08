axhub plugin 의 SKILL 작성자 역할이에요. 다음 routing failure 를 수정해야 해요.

발화: "{utterance}"
기대된 skill: {expected_skill}
실제 fired skill: {actual_skill}
failure source: {source}

skills/{expected_skill}/SKILL.md 의 현재 description:
{current_description}

skills/{expected_skill}/SKILL.md 의 현재 examples:
{current_examples}

이 발화가 {expected_skill} 로 매칭되도록 description trigger 어구 또는 examples 를 어떻게 보강하면 좋을지 1-3개 suggestion 을 JSON 으로 제시해요. intent 는 영어 action phrase 로 써요.

{
  "description_additions": ["어구1", "어구2"],
  "example_additions": [
    {"utterance": "...", "intent": "..."}
  ]
}
