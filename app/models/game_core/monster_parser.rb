module GameCore
  class MonsterParser

    attr_reader :name, :original_name

    EXCEPTION = {
      'http://lesitedontvousetesleheros.overblog.com/2014/12/520.html' =>
        [ { strength: 5, hp: 1, adjustment: 0, name: 'CHAQUE RAT CARNIVORE' } ],
      'http://lesitedontvousetesleheros.overblog.com/2014/12/536.html' =>
        [ { strength: 10, hp: 8, adjustment: 0, name: 'TÊTE-SANS-CORPS' } ],
      'http://lesitedontvousetesleheros.overblog.com/2014/12/209.html' =>
        [
          { strength: 6, hp: 10, adjustment: 0, name: 'PREMIER GNOME EMBAUMEUR' },
          { strength: 9, hp: 10, adjustment: 0, name: 'SECOND GNOME EMBAUMEUR' }
        ],
      'http://lesitedontvousetesleheros.overblog.com/2014/12/429.html' =>
        [
          { strength: 12, hp: 4, adjustment: 0, name: 'PREMIER HOMME-SABLE' },
          { strength: 13, hp: 4, adjustment: 0, name: 'SECOND HOMME-SABLE' }
        ],
      'http://lesitedontvousetesleheros.overblog.com/2014/12/302.html' =>
        [
          { strength: 14, hp: 16, adjustment: 1, name: 'SCORPION GÉANT' }
        ],
      'http://lesitedontvousetesleheros.overblog.com/2014/12/9.html' =>
        [
          { strength: 6, hp: 10, adjustment: 0, name: 'PREMIÈRE CHAUVE-SOURIS' },
          { strength: 6, hp: 12, adjustment: 0, name: 'DEUXIÈME CHAUVE-SOURIS' },
          { strength: 6, hp: 11, adjustment: 0, name: 'TROISIÈME CHAUVE-SOURIS' }
        ],
      'http://lesitedontvousetesleheros.overblog.com/2014/12/289-0.html' =>
        [
          { strength: 6, hp: 9, adjustment: 0, name: 'MONROCK' }
        ],
      'http://lesitedontvousetesleheros.overblog.com/2014/12/278-6.html' =>
        [
          { strength: 14, hp: 16, adjustment: 1, name: 'PLÉSIOSAURE' }
        ],
      'http://lesitedontvousetesleheros.overblog.com/2014/12/530-3.html' =>
        [
          { strength: 14, hp: 16, adjustment: 1, name: 'TAUPE GÉANTE' }
        ],
      'http://lesitedontvousetesleheros.overblog.com/2014/12/411-3.html' =>
        [
          { strength: 14, hp: 16, adjustment: 1, name: 'TAUPE GÉANTE' }
        ],
      'http://lesitedontvousetesleheros.overblog.com/2014/12/166-6.html' =>
        [
          { strength: 6, hp: 9, adjustment: 0, name: 'VER DES CAVERNES' }
        ],
      'http://lesitedontvousetesleheros.overblog.com/2015/01/347.html' =>
        [
          { strength: 11, hp: 13, adjustment: 0, name: 'PREMIER VOLEUR' },
          { strength: 12, hp: 10, adjustment: 0, name: 'DEUXIÈME VOLEUR' }
        ],
      'http://lesitedontvousetesleheros.overblog.com/2015/01/383.html' =>
        [
          { strength: 7, hp: 6, adjustment: 0, name: 'KOSHEN' }
        ],
      'http://lesitedontvousetesleheros.overblog.com/2015/01/458.html' =>
        [
          { strength: 12, hp: 1, adjustment: 0, name: 'GUERRIER' }
        ],
      'http://lesitedontvousetesleheros.overblog.com/2014/12/181-8.html' =>
        [
          { strength: 13, hp: 1, adjustment: 0, name: 'GUERRIER' }
        ],
      'http://lesitedontvousetesleheros.overblog.com/2014/12/15-2.html' =>
        [
          { strength: 15, hp: 1, adjustment: 0, name: 'GUERRIER' }
        ],
      'http://lesitedontvousetesleheros.overblog.com/2015/01/268.html' =>
        [
          { strength: 12, hp: 13, adjustment: 0, name: 'ALSHAYA LE NOIR' }
        ],
      'http://lesitedontvousetesleheros.overblog.com/2015/01/312.html' =>
        [
          { strength: 8, hp: 11, adjustment: 0, name: 'GUERRIER FORGE' }
        ],
      'http://lesitedontvousetesleheros.overblog.com/2015/01/333.html' =>
        [
          { strength: 15, hp: 10, adjustment: 0, name: 'MOLOCH' }
        ]
    }

    def initialize( child )
      @name = child.children.first.to_s.strip.upcase
      @original_name = name
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

    # Exception handling
    def self.exception_page?( url )
      # p EXCEPTION.keys, url
      EXCEPTION.keys.include?( url )
    end

    def self.create_monster_object_on_exceptional_page( url, section )
      EXCEPTION[ url ].each do |e|
        m = Monster.find_or_create_by!( name: e[:name], hp: e[:hp], strength: e[:strength], adjustment: e[:adjustment] )
        section.monsters << m
      end
    end
  end
end
