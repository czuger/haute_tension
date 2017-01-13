module GameCore
  class MonsterParser

    attr_reader :name, :original_name

    EXCEPTION = {
      'CHAQUE RAT' => { strength: 5, hp: 1, adjustment: 0, name: 'CHAQUE RAT' },
      'TÊTE-SANS-CORPS FORCE' => { strength: 10, hp: 8, adjustment: 0, name: 'TÊTE-SANS-CORPS' },
      'PREMIER GNOME EMBAUMEUR' => { strength: 6, hp: 10, adjustment: 0, name: 'PREMIER GNOME EMBAUMEUR' },
      'SECOND GNOME EMBAUMEUR' => { strength: 9, hp: 10, adjustment: 0, name: 'SECOND GNOME EMBAUMEUR' },
      'SECOND HOMME-SABLE' => { strength: 13, hp: 4, adjustment: 0, name: 'SECOND HOMME-SABLE' },
      'SCORPION GÉANT FOR' => { strength: 14, hp: 16, adjustment: 1, name: 'SCORPION GÉANT' },
      'DEUXIÈME CHAUVE-SOURIS' => { strength: 6, hp: 12, adjustment: 0, name: 'DEUXIÈME CHAUVE-SOURIS' },
      'MONROCK FORCE: 6 VIE: 9' => { strength: 6, hp: 9, adjustment: 0, name: 'MONROCK' }
    }

    def initialize( child )
      @name = child.children.first.to_s.strip.upcase
      @original_name = name

      if EXCEPTION.keys.include?( @name )

        @strength = EXCEPTION[ @name ][ :strength ]
        @hp = EXCEPTION[ @name ][ :hp ]
        @adjustment = EXCEPTION[ @name ][ :adjustment ]
        @original_name = @name
        @name = EXCEPTION[ @name ][ :name ]

        @exception = true
      end

    end

    def create_monster_object( section )
      # p monster
      m = Monster.find_or_create_by!( name: @name, hp: @hp.to_i, strength: @strength.to_i, adjustment: @adjustment )
      section.monsters << m
    end

    def rubbish?
      @name == '<EM> </EM>' || @name =~ /\(\d+\)/ || @name.empty?
    end

    def check!
      unless @strength && @hp
        p self
        raise 'Bad monster'
      end
    end

    def exception?
      @exception
    end

    def process_stats( child )
      # p child
      child = child.to_s.upcase
      r = child.match( /FORCE ?: ?([0-9]+)/ )
      @strength = r[1] if r

      r = child.match( /VIE ?: ?([0-9]+)/ )
      # Forteresse d'alamuth section 520 (http://www.lesitedontvousetesleheros.fr/2014/12/520.html)
      # Rats does not have life
      @hp = r[1] if r
      # @monster[ :vie ] = 1 if @monster[ :name ] == 'CHAQUE RAT'

      adjustment = 0
      r = child.match( /AJUSTEMENT DEGATS ?: ?(\+|-) ?([0-9]+)/ )
      if r
        adjustment_sign = r[1] if r
        adjustment_value = r[2] if r
        @adjustment = (adjustment_sign+adjustment_value).to_i
      end
    end
  end
end
