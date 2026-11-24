# frozen_string_literal: true

# Projected by hand from rule step and rule lose (src/hop/world.slg). Stand-in for spec-lang build output; change the spec, not this file.

module Skyhop
  class World
    # One tick of play, built up in local variables and committed as one new World at the end.
    class Step
      include Physics

      def self.call(world, pad)
        raise Over unless world.mode == :Playing

        new(world).run(pad)
      end

      def initialize(world)
        @world = world
        @stage = world.stage
        @t = world.ticks + 1
        @lines = []
        @taken = world.taken
        @crawlers = world.crawlers
        @lives = world.lives
        @mode = world.mode
      end

      def run(pad)
        was = @world.player
        at, vy = move(was, pad)
        take_gems(at)
        touch_crawlers(at, vy)
        finish(at) if @alive
        Outcome.new(World.new(stage: @stage, ticks: @t, player: @player, taken: @taken, crawlers: @crawlers,
                              lives: @lives, mode: @mode), @lines.freeze)
      end

      private

      # Carry, steer, jump, fall: returns where the feet end and the vertical speed the fall used.
      def move(was, pad)
        at = carried(was)
        vy, coyote = jumped(was, pad)
        target = Physics.walk_target(pad)
        accel = was.grounded ? GROUND_ACCEL : AIR_ACCEL
        sx = Physics.slide_x(at, Geom.approach(was.vel.x, target.x, accel), @stage.solids)
        at = at.with(x: sx.pos)
        sz = Physics.slide_z(at, Geom.approach(was.vel.z, target.z, accel), @stage.solids)
        at = at.with(z: sz.pos)
        f = Physics.fall(at, vy, @stage.solids, tops)
        at = at.with(y: f.y)
        @lines << "#{@t} land" if f.grounded && !was.grounded
        @player = Model::Player.new(at:, vel: Geom::Vec3.new(sx.vel, f.vel, sz.vel), grounded: f.grounded,
                                    coyote: f.grounded ? COYOTE_TICKS : [coyote - 1, 0].max,
                                    jump_held: pad.jump, riding: f.riding, yaw: Physics.facing(pad, was.yaw))
        [at, vy]
      end

      def tops = @stage.movers.map { |m| Placed.new(m.no, Physics.mover_box(m, @t)) }

      def carried(was)
        at = was.at
        return at if was.riding.nil?

        @stage.movers.each { |m| at = Geom.plus(at, Physics.mover_shift(m, @t)) if m.no == was.riding }
        at
      end

      def jumped(was, pad)
        vy = was.vel.y
        coyote = was.coyote
        if pad.jump && !was.jump_held && (was.grounded || was.coyote.positive?)
          vy = JUMP_SPEED
          coyote = 0
          @lines << "#{@t} jump"
        elsif !pad.jump && vy > HOP_CUT
          vy = HOP_CUT
        end
        [[vy - GRAVITY, -MAX_FALL].max, coyote]
      end

      def take_gems(at)
        @crawlers = @crawlers.transform_values { Physics.patrol(it) }.freeze
        hero = Physics.body(at)
        @stage.gems.each do |g|
          next if @taken.include?(g.no) || !Geom.touches(hero, Geom.around_centre(g.at, GEM_HALF))

          @taken = (@taken | [g.no]).freeze
          @lines << "#{@t} gem #{g.no}"
        end
      end

      def touch_crawlers(at, vy)
        @alive = true
        hero = Physics.body(at)
        @crawlers.each_value do |c|
          next unless @alive && Geom.touches(hero, Physics.crawler_box(c))

          if Physics.stomps(at.y, vy, c)
            stomp(c)
          else
            lose("hit")
          end
        end
      end

      def stomp(crawler)
        @crawlers = @crawlers.except(crawler.no).freeze
        @player = @player.with(vel: @player.vel.with(y: STOMP_BOUNCE), grounded: false, riding: nil)
        @lines << "#{@t} stomp #{crawler.no}"
      end

      def finish(at)
        return lose("fall") if at.y < KILL_Y
        return unless Geom.touches(Physics.body(at), Geom.around_feet(@stage.goal, GOAL_HALF))

        @mode = :Won
        @lines << "#{@t} won"
      end

      # `lose` joins `step`; its guard holds here because `step`'s did and nothing before it changes `mode`.
      def lose(why)
        @lines << "#{@t} #{why}"
        @lives -= 1
        @player = Model::Player.new(at: @stage.start)
        @alive = false
        return unless @lives.zero?

        @mode = :Lost
        @lines << "#{@t} lost"
      end
    end

    def step(pad) = Step.call(self, pad)
  end
end
