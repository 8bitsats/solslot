import React, { ReactElement, useEffect, useState, useCallback } from "react";
import "./reel.css";

export const Reel = ({
  index,
  target,
  spin,
  onSpinFinished = null,
}: any): ReactElement => {
  const [el, setEl] = useState(null);
  const [el2, setEl2] = useState(null);
  const [requestID, setRequestID] = useState<number | null>(null);

  const startAnimation = useCallback((
    e1: any,
    e2: any,
    target: number,
    animationFinished: any = null
  ) => {
    let e1target = e1.offsetTop + target;
    let executed = 0;
    let prevFrame = Date.now();

    function playAnimation() {
      let step = target < 0 ? 8 : 6 * ((target - executed) / target) + 2;
      const interval = Date.now() - prevFrame;
      step = step * (interval / 10);

      let e1pos = e1.offsetTop + step;
      if (e1pos > 400) {
        e1pos = e1pos - 1600;
      }

      if (target > 0) {
        executed += step;
        if (executed >= target) {
          let adjustedTarget = e1target;
          while (adjustedTarget > 400) {
            adjustedTarget = adjustedTarget - 1600;
          }

          e1pos = adjustedTarget;
        }
      }

      const e2pos = e1pos > -100 ? e1pos - 800 : e1pos + 800;

      e1.style.top = `${e1pos}px`;
      e2.style.top = `${e2pos}px`;

      if (executed < target || target < 0) {
        prevFrame = Date.now();
        const newRequestID = requestAnimationFrame(playAnimation);
        setRequestID(newRequestID);
      } else {
        animationFinished && animationFinished();
      }
    }
    const initialRequestID = requestAnimationFrame(playAnimation);
    setRequestID(initialRequestID);
  }, []);

  useEffect(() => {
    if (el && el2 && target !== null) {
      if (target < 0) {
        startAnimation(el, el2, -1, onSpinFinished);
      } else {
        //@ts-ignore
        const currentPos = el.offsetTop;
        startAnimation(
          el,
          el2,
          index * 800 + 1600 + 50 - currentPos - 80 * target,
          onSpinFinished
        );
      }
    }

    return () => {
      if (requestID !== null) {
        cancelAnimationFrame(requestID);
      }
    };
  }, [el, el2, spin, target, index, onSpinFinished, startAnimation, requestID]);

  return (
    <div className="reel">
      <div className="reel-container">
        <div
          className={`wheel wheel${index} w1`}
          ref={(el) => {
            setEl(el as any);
          }}
        ></div>
        <div
          className={`wheel wheel${index} w2`}
          ref={(el) => {
            setEl2(el as any);
          }}
        ></div>
      </div>
    </div>
  );
};
