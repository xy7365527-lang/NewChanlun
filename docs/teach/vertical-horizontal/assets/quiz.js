/**
 * quiz.js — 可复用单选测验小组件（原生 JS，无依赖）
 *
 * 用法（在课程 HTML 里，配合 course.css 的 .quiz-* 样式）：
 *
 * <div class="quiz-question">
 *   <p class="quiz-prompt">问题文本……</p>
 *   <div class="quiz-options">
 *     <button class="quiz-option" data-correct="false">
 *       选项文本
 *       <span class="quiz-explain">为什么这个不对……</span>
 *     </button>
 *     <button class="quiz-option" data-correct="true">
 *       选项文本
 *       <span class="quiz-explain">为什么这个对……</span>
 *     </button>
 *     <!-- 可以有 2~5 个选项，每个都要有 data-correct 和 .quiz-explain -->
 *   </div>
 *   <p class="quiz-status" role="status" aria-live="polite" hidden></p>
 *   <button type="button" class="quiz-retry" hidden>重新作答</button>
 * </div>
 *
 * 行为：点选任意一项 → 立即锁定该题、标出正确项、展开被点中那项
 * （以及答错时正确项）的解释、显示状态行；点"重新作答"可清空重来。
 *
 * ============================================================
 * 出题铁律（写在这里，供以后任何人加题时遵守，不是建议是约束）：
 *
 *   同一题内，所有 .quiz-option 的可见文本（不算 .quiz-explain 里
 *   藏着的解释部分）长度必须尽量一致——不能让学习者靠"哪个选项最长/
 *   最短"猜答案，也不能靠标点、括号数量之类的形式线索猜。
 *
 *   本文件在初始化时会做一次长度体检，长度差过大时在浏览器控制台
 *   warn 出来，但这只是安全网，不是强制校验（HTML 里怎么写它都会
 *   照样渲染）。出题人要自己守住这条约束，不能依赖这个警告。
 * ============================================================
 */
(function () {
  'use strict';

  var LENGTH_WARN_RATIO = 1.4; // 最长/最短 可见文本长度比超过这个值就警告
  var LENGTH_WARN_ABS = 6; // 且绝对字符数差超过这个值才警告（避免短文本也报警）

  /** 只量"肉眼可见的选项文本"，把藏着的 .quiz-explain 解释部分挖掉再量。 */
  function visibleOptionText(optionEl) {
    var clone = optionEl.cloneNode(true);
    var explain = clone.querySelector('.quiz-explain');
    if (explain && explain.parentNode) {
      explain.parentNode.removeChild(explain);
    }
    return clone.textContent.trim();
  }

  function checkOptionLengths(question, options) {
    var lengths = options.map(function (opt) {
      return visibleOptionText(opt).length;
    });
    var max = Math.max.apply(null, lengths);
    var min = Math.min.apply(null, lengths);
    if (min === 0) {
      return;
    }
    var ratio = max / min;
    if (ratio > LENGTH_WARN_RATIO && max - min > LENGTH_WARN_ABS) {
      var prompt = question.querySelector('.quiz-prompt');
      console.warn(
        '[quiz.js] 选项文本长度差异较大（' +
          min +
          ' ~ ' +
          max +
          ' 字），学习者可能靠长短猜答案。题目：',
        prompt ? prompt.textContent.trim() : question
      );
    }
  }

  function answerQuestion(question, options, chosen) {
    if (question.classList.contains('is-answered')) {
      return;
    }
    question.classList.add('is-answered');

    var chosenCorrect = chosen.getAttribute('data-correct') === 'true';

    options.forEach(function (opt) {
      opt.disabled = true;
      opt.setAttribute('aria-disabled', 'true');

      var isCorrect = opt.getAttribute('data-correct') === 'true';
      if (isCorrect) {
        opt.classList.add('is-correct');
      }
      if (opt === chosen && !chosenCorrect) {
        opt.classList.add('is-chosen-wrong');
      }

      // 展开解释：被点中的那项，以及（答错时）正确项的解释都要展开，
      // 让学习者不只知道对错，还知道"对的为什么对"。
      if (opt === chosen || isCorrect) {
        var explain = opt.querySelector('.quiz-explain');
        if (explain) {
          explain.hidden = false;
        }
      }
    });

    var status = question.querySelector('.quiz-status');
    if (status) {
      status.hidden = false;
      status.textContent = chosenCorrect
        ? '答对了。'
        : '不对——正确选项已标出，解释见下方。';
      status.classList.toggle('is-correct', chosenCorrect);
      status.classList.toggle('is-incorrect', !chosenCorrect);
    }

    var retry = question.querySelector('.quiz-retry');
    if (retry) {
      retry.hidden = false;
    }
  }

  function resetQuestion(question, options) {
    question.classList.remove('is-answered');
    options.forEach(function (opt) {
      opt.disabled = false;
      opt.removeAttribute('aria-disabled');
      opt.classList.remove('is-correct', 'is-chosen-wrong');
      var explain = opt.querySelector('.quiz-explain');
      if (explain) {
        explain.hidden = true;
      }
    });

    var status = question.querySelector('.quiz-status');
    if (status) {
      status.hidden = true;
      status.textContent = '';
      status.classList.remove('is-correct', 'is-incorrect');
    }

    var retry = question.querySelector('.quiz-retry');
    if (retry) {
      retry.hidden = true;
    }
  }

  function initQuestion(question) {
    var options = Array.prototype.slice.call(
      question.querySelectorAll('.quiz-option')
    );
    if (options.length === 0) {
      return;
    }

    checkOptionLengths(question, options);

    options.forEach(function (opt) {
      var explain = opt.querySelector('.quiz-explain');
      if (explain) {
        explain.hidden = true;
      }
      opt.addEventListener('click', function () {
        answerQuestion(question, options, opt);
      });
    });

    var retry = question.querySelector('.quiz-retry');
    if (retry) {
      retry.hidden = true;
      retry.addEventListener('click', function () {
        resetQuestion(question, options);
        options[0].focus();
      });
    }
  }

  function init() {
    var questions = document.querySelectorAll('.quiz-question');
    Array.prototype.forEach.call(questions, initQuestion);
  }

  if (document.readyState === 'loading') {
    document.addEventListener('DOMContentLoaded', init);
  } else {
    init();
  }
})();
